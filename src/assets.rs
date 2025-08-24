use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets/"]   // everything under this is embedded at compile time
pub struct Assets;

pub const ICON_SETTINGS: &str = "ui/settings.png";
pub const ICON_IMAGE_FILTER: &str = "ui/gallery.png";
pub const ICON_TRAY: &str = "icons/tray.png";

pub fn get_bytes(path: &str) -> Option<Cow<'static, [u8]>> {
    Assets::get(path).map(|f| f.data)
}

pub fn icon_data_from_png(bytes: &[u8]) -> Option<eframe::egui::IconData> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(eframe::egui::IconData { rgba: img.into_raw(), width: w, height: h })
}

pub fn tray_icon_from_png(bytes: &[u8]) -> anyhow::Result<tray_icon::Icon> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h)
        .map_err(|e| anyhow::anyhow!("icon from rgba: {e}"))
}

pub fn load_texture_from_asset(ctx: &egui::Context, asset_path: &str) -> Option<egui::TextureHandle> {
    let bytes = crate::assets::get_bytes(asset_path)?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img);
    Some(ctx.load_texture(asset_path.to_string(), color, egui::TextureOptions::LINEAR))
}