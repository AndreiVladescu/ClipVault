use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose, Engine as _};
use blake3::Hash;
use chrono::{DateTime, Utc};
use png::{ColorType, Decoder, Encoder};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    thread,
    time::Duration,
};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    ImageBase64(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub ts: DateTime<Utc>,
    pub content: ClipboardContent,
}

pub fn image_to_base64(img: &ImageData) -> String {
    let mut png_bytes = Vec::new();
    let mut enc = Encoder::new(&mut png_bytes, img.width as u32, img.height as u32);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&img.bytes)
        .unwrap();
    general_purpose::STANDARD.encode(png_bytes)
}

pub fn base64_to_imagedata(b64: &str) -> anyhow::Result<ImageData<'_>> {
    let bytes = general_purpose::STANDARD.decode(b64)?;
    let cursor = std::io::Cursor::new(bytes);
    let mut reader = Decoder::new(cursor).read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    Ok(ImageData {
        width: info.width as usize,
        height: info.height as usize,
        bytes: buf[..info.buffer_size()].to_vec().into(),
    })
}

fn read_clipboard() -> Result<Option<ClipboardContent>, arboard::Error> {
    let mut clipboard = Clipboard::new()?;

    if let Ok(txt) = clipboard.get_text() {
        return Ok(Some(ClipboardContent::Text(txt)));
    }
    if let Ok(img) = clipboard.get_image() {
        return Ok(Some(ClipboardContent::ImageBase64(image_to_base64(&img))));
    }
    Ok(None)
}

fn set_clipboard(content: &ClipboardContent) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    match content {
        ClipboardContent::Text(t) => clipboard.set_text(t.clone()),
        ClipboardContent::ImageBase64(b64) => {
            let img = base64_to_imagedata(b64).map_err(|_| arboard::Error::ContentNotAvailable)?;
            clipboard.set_image(img)
        }
    }
}

fn append_to_history(entry: &ClipboardEntry) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(HISTORY_PATH)?;
    serde_json::to_writer(&mut file, entry)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn load_history() -> anyhow::Result<Vec<ClipboardEntry>> {
    let file = match OpenOptions::new().read(true).open(HISTORY_PATH) {
        Ok(f) => f,
        Err(_) => return Ok(Vec::new()), // first run means empty history
    };
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: ClipboardEntry = serde_json::from_str(&line)?;
        out.push(entry);
    }
    Ok(out)
}

fn clipboard_entry_hash(c: &ClipboardContent) -> Hash {
    match c {
        ClipboardContent::Text(text_string) => blake3::hash(text_string.as_bytes()),
        ClipboardContent::ImageBase64(b64_image) => blake3::hash(b64_image.as_bytes()),
    }
}

const HISTORY_PATH: &str = "history.jsonl";

fn spawn_watcher(
    tx: crossbeam::channel::Sender<ClipboardEntry>,
    mut last_hash: Option<Hash>,
) {
    thread::spawn(move || {
        loop {
            match read_clipboard() {
                Ok(Some(content)) => {
                    let h = clipboard_entry_hash(&content);
                    if Some(h) != last_hash {
                        let entry = ClipboardEntry { ts: Utc::now(), content: content.clone() };
                        // persist to disk
                        let _ = append_to_history(&entry);
                        // send to UI
                        let _ = tx.send(entry);
                        last_hash = Some(h);
                    }
                }
                Ok(None) => {}        // nothing on clipboard / unsupported
                Err(_e) => {          // clipboard temporarily unavailable? ignore and retry
                    // eprintln!("clipboard read error: {e:?}");
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

struct ClipApp {
    rx: crossbeam::channel::Receiver<ClipboardEntry>,
    history: Vec<ClipboardEntry>,
    filter: String,
}


impl eframe::App for ClipApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // pull any new entries from the watcher
        for entry in self.rx.try_iter() {
            self.history.push(entry);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ClipVault");
                ui.separator();
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let q = self.filter.to_lowercase();
                for entry in self.history.iter().rev() {
                    if !q.is_empty() {
                        if let ClipboardContent::Text(t) = &entry.content {
                            if !t.to_lowercase().contains(&q) {
                                continue;
                            }
                        } else {
                            // filter only applies to text for now
                        }
                    }

                    ui.horizontal(|ui| {
                        if ui.button("📋").on_hover_text("Restore to clipboard").clicked() {
                            let _ = set_clipboard(&entry.content);
                        }

                        match &entry.content {
                            ClipboardContent::Text(t) => {
                                let mut t = t.clone();
                                if let Some((idx, _)) = t.match_indices('\n').nth(4) {
                                    t = t[..idx].to_string();
                                    t.push_str("\n...");
                                }
                                ui.label(egui::RichText::new(t));
                            }
                            ClipboardContent::ImageBase64(b64) => {
                                ui.label(format!("<image {} bytes>", b64.len()));
                                // (Thumbnails later)
                            }
                        }
                    });
                }
            });
        });
    }
}

fn main() -> anyhow::Result<()> {
    // load existing history
    let history = load_history()?;
    let last_hash = history.last().map(|e| clipboard_entry_hash(&e.content));

    // channel for watcher -> UI
    let (tx, rx) = crossbeam::channel::unbounded();
    spawn_watcher(tx, last_hash);

    let options = eframe::NativeOptions::default();

    // NOTE: closure must return Result<Box<dyn App>, _>
    let res = eframe::run_native(
        "ClipVault",
        options,
        Box::new(|_cc| {
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(
                Box::new(ClipApp {
                    rx,
                    history,
                    filter: String::new(),
                })
            )
        }),
    );

    if let Err(e) = res {
        eprintln!("eframe error: {e}");
    }
    Ok(())
}
