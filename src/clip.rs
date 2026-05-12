use arboard::Clipboard;
use blake3::Hash;
use chrono::Utc;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use crossbeam::channel::Sender;
use std::{io, thread, time::Duration};

use crate::img::{image_to_png, png_to_imagedata};
use crate::types::{ClipboardContent, ClipboardEntry};

pub fn clipboard_entry_hash(c: &ClipboardContent) -> Hash {
    match c {
        ClipboardContent::Text(s) => blake3::hash(s.as_bytes()),
        ClipboardContent::Image(bytes) => blake3::hash(bytes),
    }
}

pub fn content_key(c: &ClipboardContent) -> String {
    clipboard_entry_hash(c).to_hex().to_string()
}

pub fn set_clipboard(content: &ClipboardContent) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    match content {
        ClipboardContent::Text(t) => clipboard.set_text(t.clone()),
        ClipboardContent::Image(bytes) => {
            let img = png_to_imagedata(bytes).map_err(|_| arboard::Error::ContentNotAvailable)?;
            clipboard.set_image(img)
        }
    }
}

fn try_capture(clipboard: &mut Clipboard, tx: &Sender<ClipboardEntry>, last_hash: &mut Option<Hash>) {
    let content = if let Ok(txt) = clipboard.get_text() {
        ClipboardContent::Text(txt)
    } else if let Ok(img) = clipboard.get_image() {
        ClipboardContent::Image(image_to_png(&img))
    } else {
        return;
    };

    let h = clipboard_entry_hash(&content);
    if Some(h) != *last_hash {
        let _ = tx.send(ClipboardEntry { ts: Utc::now(), content });
        *last_hash = Some(h);
    }
}

struct ClipWatcher {
    tx: Sender<ClipboardEntry>,
    last_hash: Option<Hash>,
    clipboard: Clipboard,
}

impl ClipboardHandler for ClipWatcher {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        try_capture(&mut self.clipboard, &self.tx, &mut self.last_hash);
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("clipboard error: {error}");
        CallbackResult::Next
    }
}

fn polling_watcher(tx: Sender<ClipboardEntry>, mut last_hash: Option<Hash>) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("clipboard polling init: {e}");
            return;
        }
    };
    loop {
        try_capture(&mut clipboard, &tx, &mut last_hash);
        thread::sleep(Duration::from_millis(500));
    }
}

pub fn spawn_watcher(tx: Sender<ClipboardEntry>, last_hash: Option<Hash>) {
    thread::spawn(move || {
        let clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("clipboard init: {e}");
                return;
            }
        };

        let watcher = ClipWatcher {
            tx: tx.clone(),
            last_hash,
            clipboard,
        };

        match Master::new(watcher) {
            Ok(mut master) => {
                if let Err(e) = master.run() {
                    // X11 not available (e.g. pure Wayland): fall back to polling
                    eprintln!("event-driven clipboard unavailable ({e}), falling back to polling");
                    polling_watcher(tx, last_hash);
                }
            }
            Err(e) => {
                eprintln!("clipboard-master init failed ({e}), falling back to polling");
                polling_watcher(tx, last_hash);
            }
        }
    });
}
