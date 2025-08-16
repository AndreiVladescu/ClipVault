use arboard::ImageData;
use base64::{engine::general_purpose, Engine as _};
use png::{ColorType, Decoder, Encoder};

pub fn image_to_base64(img: &ImageData) -> String {
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut enc: Encoder<'static, &mut Vec<u8>> =
        Encoder::new(&mut png_bytes, img.width as u32, img.height as u32);
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
