use arboard::{Clipboard, ImageData};
use blake3::Hash;
use chrono::Utc;
use crossbeam::channel::Sender;
use std::{thread, time::Duration};

use crate::img::{base64_to_imagedata, image_to_base64};
use crate::types::{ClipboardContent, ClipboardEntry};

pub fn clipboard_entry_hash(c: &ClipboardContent) -> Hash {
    match c {
        ClipboardContent::Text(text_string) => blake3::hash(text_string.as_bytes()),
        ClipboardContent::ImageBase64(b64_image) => blake3::hash(b64_image.as_bytes()),
    }
}

pub fn content_key(c: &ClipboardContent) -> String {
    clipboard_entry_hash(c).to_hex().to_string()
}

pub fn read_clipboard() -> Result<Option<ClipboardContent>, arboard::Error> {
    let mut clipboard: Clipboard = Clipboard::new()?;

    if let Ok(txt) = clipboard.get_text() {
        return Ok(Some(ClipboardContent::Text(txt)));
    }
    if let Ok(img) = clipboard.get_image() {
        return Ok(Some(ClipboardContent::ImageBase64(image_to_base64(&img))));
    }
    Ok(None)
}

pub fn set_clipboard(content: &ClipboardContent) -> Result<(), arboard::Error> {
    let mut clipboard: Clipboard = Clipboard::new()?;
    match content {
        ClipboardContent::Text(t) => clipboard.set_text(t.clone()),
        ClipboardContent::ImageBase64(b64) => {
            let img: ImageData<'_> =
                base64_to_imagedata(b64).map_err(|_| arboard::Error::ContentNotAvailable)?;
            clipboard.set_image(img)
        }
    }
}

pub fn spawn_watcher(tx: Sender<ClipboardEntry>, mut last_hash: Option<Hash>) {
    thread::spawn(move || {
        loop {
            match read_clipboard() {
                Ok(Some(content)) => {
                    let h: Hash = clipboard_entry_hash(&content);
                    if Some(h) != last_hash {
                        let entry: ClipboardEntry = ClipboardEntry {
                            ts: Utc::now(),
                            content: content.clone(),
                        };
                        let _ = tx.send(entry);
                        last_hash = Some(h);
                    }
                }
                Ok(None) => {}
                Err(_e) => {
                    eprintln!("clipboard read error: {_e:?}");
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
