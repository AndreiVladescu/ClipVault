use arboard::ImageData;
use png::{ColorType, Decoder, Encoder};

pub fn image_to_png(img: &ImageData) -> Vec<u8> {
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut enc: Encoder<'static, &mut Vec<u8>> =
        Encoder::new(&mut png_bytes, img.width as u32, img.height as u32);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&img.bytes)
        .unwrap();
    png_bytes
}

pub fn png_to_imagedata(bytes: &[u8]) -> anyhow::Result<ImageData<'static>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut reader = Decoder::new(cursor).read_info()?;
    let mut buf: Vec<u8> = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    Ok(ImageData {
        width: info.width as usize,
        height: info.height as usize,
        bytes: buf[..info.buffer_size()].to_vec().into(),
    })
}
