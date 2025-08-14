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
    collections::{HashMap, HashSet}
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

struct ClipApp {
    rx: crossbeam::channel::Receiver<ClipboardEntry>,
    history: Vec<ClipboardEntry>,
    seen: HashSet<String>,
    filter: String,
    tex_cache: HashMap<String, egui::TextureHandle>,
}

#[derive(Clone)]
struct Agg {
    content: ClipboardContent,
    created_ts: DateTime<Utc>,
    last_ts: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum LogRec {
    Put { key: String, ts: DateTime<Utc>, content: ClipboardContent },
    Touch { key: String, ts: DateTime<Utc> },
}


const HISTORY_PATH: &str = "history.jsonl";
pub fn compact_history_log() -> anyhow::Result<()> {
    use std::collections::HashMap;

    let path = std::path::Path::new(HISTORY_PATH);
    if !path.exists() { return Ok(()); }

    let file = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(file);
    let mut map: HashMap<String, Agg> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let rec: LogRec = serde_json::from_str(&line)?;
        match rec {
            LogRec::Put { key, ts, content } => {
                map.entry(key).and_modify(|a| {
                    if ts < a.created_ts { a.created_ts = ts; }
                    if ts > a.last_ts    { a.last_ts    = ts; }
                }).or_insert(Agg { content, created_ts: ts, last_ts: ts });
            }
            LogRec::Touch { key, ts } => {
                if let Some(a) = map.get_mut(&key) {
                    if ts > a.last_ts { a.last_ts = ts; }
                }
            }
        }
    }

    // write compacted file
    let tmp = path.with_extension("jsonl.tmp");
    {
        let mut out = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&tmp)?;
        // newest first
        let mut items: Vec<_> = map.into_iter().collect();
        items.sort_by(|a,b| b.1.last_ts.cmp(&a.1.last_ts));

        for (key, agg) in items {
            let put = LogRec::Put { key: key.clone(), ts: agg.created_ts, content: agg.content.clone() };
            serde_json::to_writer(&mut out, &put)?; out.write_all(b"\n")?;
            if agg.last_ts > agg.created_ts {
                let touch = LogRec::Touch { key, ts: agg.last_ts };
                serde_json::to_writer(&mut out, &touch)?; out.write_all(b"\n")?;
            }
        }
        out.flush()?;
    }
    let _ = std::fs::remove_file(path);
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn ensure_texture_for_b64(
    cache: &mut HashMap<String, egui::TextureHandle>,
    ctx: &egui::Context,
    key: &str,
    b64: &str,
) {
    if cache.contains_key(key) { return; }

    if let Ok(img) = base64_to_imagedata(b64) {
        let color: egui::ColorImage = egui::ColorImage::from_rgba_unmultiplied(
            [img.width, img.height],
            &img.bytes,
        );
        let tex: egui::TextureHandle = ctx.load_texture(
            format!("thumb-{key}"),
            color,
            egui::TextureOptions::LINEAR, // smooth when scaled down
        );
        cache.insert(key.to_owned(), tex);
    }
}

pub fn image_to_base64(img: &ImageData) -> String {
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut enc: Encoder<'static, &mut Vec<u8>> = Encoder::new(&mut png_bytes, img.width as u32, img.height as u32);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&img.bytes)
        .unwrap();
    general_purpose::STANDARD.encode(png_bytes)
}

pub fn base64_to_imagedata(b64: &str) -> anyhow::Result<ImageData<'_>> {
    let bytes: Vec<u8> = general_purpose::STANDARD.decode(b64)?;
    let cursor: std::io::Cursor<Vec<u8>> = std::io::Cursor::new(bytes);
    let mut reader: png::Reader<std::io::Cursor<Vec<u8>>> = Decoder::new(cursor).read_info()?;
    let mut buf: Vec<u8> = vec![0; reader.output_buffer_size()];
    let info: png::OutputInfo = reader.next_frame(&mut buf)?;
    Ok(ImageData {
        width: info.width as usize,
        height: info.height as usize,
        bytes: buf[..info.buffer_size()].to_vec().into(),
    })
}

fn append_put(key: &str, content: &ClipboardContent, ts: DateTime<Utc>) -> anyhow::Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(HISTORY_PATH)?;
    serde_json::to_writer(&mut f, &LogRec::Put { key: key.to_string(), ts, content: content.clone() })?;
    f.write_all(b"\n")?;
    Ok(())
}

fn append_touch(key: &str, ts: DateTime<Utc>) -> anyhow::Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(HISTORY_PATH)?;
    serde_json::to_writer(&mut f, &LogRec::Touch { key: key.to_string(), ts })?;
    f.write_all(b"\n")?;
    Ok(())
}

fn content_key(c: &ClipboardContent) -> String {
    clipboard_entry_hash(c).to_hex().to_string()
}

fn read_clipboard() -> Result<Option<ClipboardContent>, arboard::Error> {
    let mut clipboard: Clipboard = Clipboard::new()?;

    if let Ok(txt) = clipboard.get_text() {
        return Ok(Some(ClipboardContent::Text(txt)));
    }
    if let Ok(img) = clipboard.get_image() {
        return Ok(Some(ClipboardContent::ImageBase64(image_to_base64(&img))));
    }
    Ok(None)
}

fn set_clipboard(content: &ClipboardContent) -> Result<(), arboard::Error> {
    let mut clipboard: Clipboard = Clipboard::new()?;
    match content {
        ClipboardContent::Text(t) => clipboard.set_text(t.clone()),
        ClipboardContent::ImageBase64(b64) => {
            let img: ImageData<'_> = base64_to_imagedata(b64).map_err(|_| arboard::Error::ContentNotAvailable)?;
            clipboard.set_image(img)
        }
    }
}

fn load_history_mru() -> anyhow::Result<Vec<ClipboardEntry>> {
    use std::collections::HashMap;
    let file = match OpenOptions::new().read(true).open(HISTORY_PATH) {
        Ok(f) => f, Err(_) => return Ok(Vec::new()),
    };
    let reader = BufReader::new(file);

    let mut map: HashMap<String, (ClipboardContent, DateTime<Utc>)> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let rec: LogRec = serde_json::from_str(&line)?;
        match rec {
            LogRec::Put { key, ts, content } => {
                let e = map.entry(key).or_insert((content, ts));
                if ts > e.1 { e.1 = ts; }
                else { e.0 = e.0.clone(); } // content already set
                e.0 = e.0.clone();
            }
            LogRec::Touch { key, ts } => {
                if let Some(e) = map.get_mut(&key) {
                    e.1 = ts;
                } // else: touched before put (rare) — ignore or store a placeholder
            }
        }
    }

    let mut v: Vec<ClipboardEntry> = map
        .into_iter()
        .map(|(_k, (content, ts))| ClipboardEntry { ts, content })
        .collect();
    v.sort_by(|a, b| b.ts.cmp(&a.ts)); // newest first
    Ok(v)
}


fn clipboard_entry_hash(c: &ClipboardContent) -> Hash {
    match c {
        ClipboardContent::Text(text_string) => blake3::hash(text_string.as_bytes()),
        ClipboardContent::ImageBase64(b64_image) => blake3::hash(b64_image.as_bytes()),
    }
}

fn spawn_watcher(
    tx: crossbeam::channel::Sender<ClipboardEntry>,
    mut last_hash: Option<Hash>,
) {
    thread::spawn(move || {
        loop {
            match read_clipboard() {
                Ok(Some(content)) => {
                    let h: Hash = clipboard_entry_hash(&content);
                    if Some(h) != last_hash {
                        let entry: ClipboardEntry = ClipboardEntry { ts: Utc::now(), content: content.clone() };
                        // send to UI
                        let _ = tx.send(entry);
                        last_hash = Some(h);
                    }
                }
                Ok(None) => {}        // nothing on clipboard / unsupported
                Err(_e) => {          
                    // clipboard temporarily unavailable? ignore and retry
                    eprintln!("clipboard read error: {_e:?}");
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
impl eframe::App for ClipApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use chrono::Utc;
        
        while let Ok(mut entry) = self.rx.try_recv() {
            let key = content_key(&entry.content);

            if self.seen.contains(&key) {
                // TOUCH: update ts, bump to front (end of vec shown in reverse)
                entry.ts = Utc::now();
                if let Some(pos) = self.history.iter().position(|e| content_key(&e.content) == key) {
                    let mut existing = self.history.remove(pos);
                    existing.ts = entry.ts;
                    self.history.push(existing);
                } else {
                    // If not present (e.g., after filtering / truncation), just push
                    self.history.push(entry.clone());
                }
                let _ = append_touch(&key, entry.ts);
            } else {
                // PUT: first time we see this content
                self.seen.insert(key.clone());
                self.history.push(entry.clone());
                let _ = append_put(&key, &entry.content, entry.ts);
            }
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ClipVault");
                ui.separator();
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter);
            });
        });

        let mut pending_restore: Option<ClipboardEntry> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let q = self.filter.to_lowercase();

                for idx in (0..self.history.len()).rev() {
                    let entry = self.history[idx].clone();
                    if !q.is_empty() {
                        if let ClipboardContent::Text(t) = &entry.content {
                            if !t.to_lowercase().contains(&q) {
                                continue;
                            }
                        }
                    }
                    let (_key, tex_opt) = match &entry.content {
                        ClipboardContent::ImageBase64(b64) => {
                            let k = content_key(&entry.content);
                            ensure_texture_for_b64(&mut self.tex_cache, ctx, &k, b64);
                            let handle = self.tex_cache.get(&k).cloned();
                            (Some(k), handle)
                        }
                        _ => (None, None),
                    };

                    ui.horizontal(|ui| {
                        if ui.button("📋").on_hover_text("Restore to clipboard").clicked() {
                            pending_restore = Some(entry.clone());
                        }

                        ui.label(
                            egui::RichText::new(format!("[{}]", entry.ts.format("%H:%M:%S")))
                                .monospace()
                                .color(egui::Color32::GRAY),
                        );

                        match (&entry.content, tex_opt) {
                            (ClipboardContent::Text(t), _) => {
                                let mut s = t.clone();
                                if let Some((cut, _)) = s.match_indices('\n').nth(4) {
                                    s.truncate(cut);
                                    s.push_str("\n…");
                                }
                                ui.label(egui::RichText::new(s));
                            }
                            (ClipboardContent::ImageBase64(b64), Some(tex)) => {
                                // Keep aspect; max width 128
                                let [w, h] = tex.size();
                                let (w, h) = (w as f32, h as f32);
                                let max_w = 128.0;
                                let scale = (max_w / w).min(1.0);
                                let w_scale = w * scale as f32;
                                let h_scale = h * scale as f32;
                                // ui.image(&tex.id(), egui::vec2(w_scale, h_scale));
                                ui.image((tex.id(), egui::vec2(w_scale, h_scale)));
                                ui.label(format!("({} bytes)", b64.len()));
                            }
                            (ClipboardContent::ImageBase64(b64), None) => {
                                ui.label(format!("<image {} bytes>", b64.len()));
                            }
                        }
                    });
                }
            });
        });         

        if let Some(entry) = pending_restore {
            let _ = set_clipboard(&entry.content);
            let key = content_key(&entry.content);
            let now = Utc::now();

            if self.seen.contains(&key) {
                if let Some(pos) = self.history.iter().position(|e| content_key(&e.content) == key) {
                    let mut existing = self.history.remove(pos);
                    existing.ts = now;
                    self.history.push(existing);
                } else {
                    self.history.push(ClipboardEntry { ts: now, content: entry.content.clone() });
                }
                let _ = append_touch(&key, now);
            } else {
                self.seen.insert(key.clone());
                let new_entry = ClipboardEntry { ts: now, content: entry.content.clone() };
                self.history.push(new_entry.clone());
                let _ = append_put(&key, &new_entry.content, now);
            }
        }
    }
}


fn main() -> anyhow::Result<()> {
    if let Err(e) = compact_history_log() {
        eprintln!("Compaction failed: {e}");
    }

    let history = load_history_mru()?;
    let last_hash = history.last().map(|e| clipboard_entry_hash(&e.content));
    let seen: std::collections::HashSet<String> =
        history.iter().map(|e| content_key(&e.content)).collect();

    let (tx, rx) = crossbeam::channel::unbounded();
    spawn_watcher(tx, last_hash);

    let options = eframe::NativeOptions::default();
    let res = eframe::run_native(
        "ClipVault",
        options,
        Box::new(|_cc| {
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(
                Box::new(ClipApp {
                    rx,
                    history,
                    seen,
                    filter: String::new(),
                    tex_cache: HashMap::new(),
                })
            )
        }),
    );

    if let Err(e) = res { eprintln!("eframe error: {e}"); }
    Ok(())
}
