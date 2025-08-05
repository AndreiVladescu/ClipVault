use arboard::{Clipboard, ImageData};
use std::io::Write;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use png::{Encoder, ColorType, Decoder};
use base64::{engine::general_purpose, Engine as _};

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

pub fn base64_to_imagedata(b64: &str) -> anyhow::Result<ImageData> {
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


fn main() -> anyhow::Result<()> {
    let current_clipboard = read_clipboard()?;
    let clipboard = match current_clipboard {
        Some(c) => c,
        None => {
            println!("Clipboard is empty or unsupported type");
            return Ok(());
        }
    };

    match &clipboard {
        ClipboardContent::Text(text_string) => println!("Text: {}", text_string),
        ClipboardContent::ImageBase64(image_string) => println!("Image (base-64, {} bytes:\n{})", image_string.len(), image_string),
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("history.jsonl")?;
    serde_json::to_writer(&mut file, &ClipboardEntry { ts: Utc::now(), content: clipboard.clone() })?;
    file.write_all(b"\n")?;

    set_clipboard(&clipboard)?;
    println!("Wrote the same item back to the clipboard.");

    Ok(())
}