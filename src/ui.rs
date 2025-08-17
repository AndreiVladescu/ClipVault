use crate::tray;
use crate::tray::TrayEvent;

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use chrono::Utc;
use crossbeam::channel::Receiver;
use egui::{RichText, StrokeKind, Align2};

use crate::clip::{content_key, set_clipboard};
use crate::img::base64_to_imagedata;
use crate::paths::history_path;
use crate::storage::{append_put, append_touch};
use crate::types::{ClipboardContent, ClipboardEntry, HotkeyMsg};
use crate::crypto::{derivate_crypto_params, decrypt_small_file};

pub struct ClipAppLocked {
    passphrase: String,
    key: [u8; 32],
    nonce: [u8; 24],
}

impl ClipAppLocked {
    pub fn new() -> Self {
        Self {
            passphrase: String::new(),
            key: [0; 32],
            nonce: [0; 24],
        }
    }

    pub fn set_crypto_params(&mut self) {
        let (key, nonce) = derivate_crypto_params(self.passphrase.clone());
        self.key = key;
        self.nonce = nonce;
        self.passphrase.clear();
    }

    pub fn try_decrypt_history(&self) -> anyhow::Result<()> {
        return decrypt_small_file(
            history_path().to_str().unwrap(),
            history_path().with_extension("decrypted.json").to_str().unwrap(),
            &self.key,
            &self.nonce,
        );
    }
}

impl eframe::App for ClipAppLocked {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            
            ui.label(
                egui::RichText::new(
                    "ClipVault is locked.\n\nTo unlock you need to enter the passphrase.",
                )
                .size(14.0),
            );
            ui.separator();
            ui.text_edit_singleline(&mut self.passphrase);
            let passphrase_button =
                egui::Button::new(RichText::new("Unlock").size(16.0)).corner_radius(6.0);
            ui.add(passphrase_button).clicked().then(|| {
                if self.passphrase.is_empty() {
                    // TODO Show error message
                } else {
                    self.set_crypto_params();
                    if (self.try_decrypt_history()).is_ok() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        // TODO Show error message
                        println!("Failed to decrypt history with the provided passphrase.");
                    }
                }
            });
        });


    }
}

pub struct ClipApp {
    tray: std::sync::Arc<tray::Tray>,
    rx: crossbeam::channel::Receiver<ClipboardEntry>,
    history: Vec<ClipboardEntry>,
    seen: HashSet<String>,
    filter: String,
    tex_cache: HashMap<String, egui::TextureHandle>,

    hotkey_rx: Receiver<HotkeyMsg>,
    window_visible: bool,
    show_settings: bool,
    show_timestamps: bool,
}

impl ClipApp {
    pub fn new(
        tray: std::sync::Arc<tray::Tray>,
        rx: crossbeam::channel::Receiver<ClipboardEntry>,
        history: Vec<ClipboardEntry>,
        seen: HashSet<String>,
        hotkey_rx: Receiver<HotkeyMsg>,
    ) -> Self {
        Self {
            tray,
            rx,
            history,
            seen,
            filter: String::new(),
            tex_cache: HashMap::new(),
            show_settings: false,
            show_timestamps: false,
            hotkey_rx,
            window_visible: false,
        }
    }

    fn show_main(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.window_visible = true;
    }

    fn hide_main(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.window_visible = false;
    }

    fn toggle_main(&mut self, ctx: &egui::Context) {
        if self.window_visible {
            self.hide_main(ctx)
        } else {
            self.show_main(ctx)
        }
    }
}

fn load_image_from_path(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
    let path: &Path = Path::new(path);
    let img: image::DynamicImage = image::open(path).ok()?;
    let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = img.to_rgba8();

    let (width, height) = img.dimensions();
    let pixels: &Vec<u8> = img.as_raw();

    let color_image: egui::ColorImage =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels);

    Some(ctx.load_texture(
        path.to_string_lossy(),
        color_image,
        egui::TextureOptions::LINEAR,
    ))
}

fn ensure_texture_for_b64(
    cache: &mut HashMap<String, egui::TextureHandle>,
    ctx: &egui::Context,
    key: &str,
    b64: &str,
) {
    if cache.contains_key(key) {
        return;
    }

    if let Ok(img) = base64_to_imagedata(b64) {
        let color: egui::ColorImage =
            egui::ColorImage::from_rgba_unmultiplied([img.width, img.height], &img.bytes);
        let tex: egui::TextureHandle = ctx.load_texture(
            format!("thumb-{key}"),
            color,
            egui::TextureOptions::LINEAR, // smooth when scaled down
        );
        cache.insert(key.to_owned(), tex);
    }
}

fn clickable_row(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let btn: egui::Button<'_> = egui::Button::new(egui::RichText::new(text)).frame(false);
    let resp: egui::Response = ui
        .add_sized([ui.available_width(), ui.spacing().interact_size.y], btn)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Click to copy");
    let rounding: egui::CornerRadius = egui::CornerRadius::same(6);
    let visuals: &egui::Visuals = ui.visuals();
    let hover_stroke: egui::Stroke =
        egui::Stroke::new(1.2, visuals.widgets.hovered.fg_stroke.color);
    let idle_stroke: egui::Stroke = egui::Stroke::new(
        0.5,
        visuals
            .widgets
            .inactive
            .fg_stroke
            .color
            .gamma_multiply(0.25),
    );
    let focus_stroke: egui::Stroke = egui::Stroke::new(2.0, visuals.selection.stroke.color);

    let stroke = if resp.has_focus() {
        focus_stroke
    } else if resp.hovered() {
        hover_stroke
    } else {
        idle_stroke
    };

    let stroke_kind: StrokeKind = StrokeKind::Inside;
    let rect: egui::Rect = resp.rect.expand(2.0);
    ui.painter()
        .rect_stroke(rect, rounding, stroke, stroke_kind);

    resp
}

impl eframe::App for ClipApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.hide_main(ctx);
            return;
        }

        while let Ok(msg) = self.hotkey_rx.try_recv() {
            match msg {
                HotkeyMsg::ToggleWindow => self.toggle_main(ctx),
            }
        }

        match self.tray.try_recv() {
            TrayEvent::OpenRequested => self.show_main(ctx),
            TrayEvent::QuitRequested => {
                // request close
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            TrayEvent::None => {}
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        while let Ok(mut entry) = self.rx.try_recv() {
            let key = content_key(&entry.content);
            if self.seen.contains(&key) {
                entry.ts = Utc::now();
                if let Some(pos) = self
                    .history
                    .iter()
                    .position(|e| content_key(&e.content) == key)
                {
                    let mut existing = self.history.remove(pos);
                    existing.ts = entry.ts;
                    self.history.push(existing);
                } else {
                    self.history.push(entry.clone());
                }
                let _ = append_touch(&key, entry.ts);
            } else {
                self.seen.insert(key.clone());
                self.history.push(entry.clone());
                let _ = append_put(&key, &entry.content, entry.ts);
            }
        }

        // Top panel
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("ClipVault").size(24.0));
                ui.separator();
                ui.label(egui::RichText::new("Filter").size(18.0));
                ui.text_edit_singleline(&mut self.filter);
                if let Some(tex) = load_image_from_path(ctx, "img/settings.png") {
                    let size = egui::vec2(24.0, 24.0);
                    let sized_tex = egui::load::SizedTexture { id: tex.id(), size };
                    let settings_icon = egui::ImageButton::new(sized_tex).corner_radius(4.0);
                    if ui.add(settings_icon.frame(true)).clicked() {
                        self.show_settings = !self.show_settings;
                    }
                } else {
                    println!("Failed to load settings icon image");
                }
            });
        });

        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.show_timestamps, "Show timestamps");
                    ui.button("Clear history").clicked().then(|| {
                        self.history.clear();
                        self.seen.clear();
                        let _ = std::fs::remove_file(history_path());
                    });
                });
        }

        let mut pending_restore: Option<ClipboardEntry> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(300.0);
                });

                let q: String = self.filter.to_lowercase();
                for idx in (0..self.history.len()).rev() {
                    let entry: ClipboardEntry = self.history[idx].clone();
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
                        ui.set_max_width(500.0);
                        if self.show_timestamps {
                            ui.label(
                                egui::RichText::new(format!("[{}]", entry.ts.format("%H:%M:%S")))
                                    .monospace()
                                    .color(egui::Color32::GRAY),
                            );
                        }

                        match (&entry.content, tex_opt) {
                            (ClipboardContent::Text(t), _) => {
                                let display_text = {
                                    let mut s = t.clone();
                                    if let Some((cut, _)) = s.match_indices('\n').nth(4) {
                                        s.truncate(cut);
                                        s.push_str("\n…");
                                    }
                                    s
                                };

                                let resp = clickable_row(ui, &display_text);

                                if resp.clicked() {
                                    pending_restore = Some(entry.clone());
                                }
                            }
                            (ClipboardContent::ImageBase64(_), Some(tex)) => {
                                let [w, h] = tex.size();
                                let (w, h) = (w as f32, h as f32);
                                let max_w = 512.0;
                                let scale = (max_w / w).min(1.0);
                                let size = egui::vec2(w * scale, h * scale);
                                let sized = egui::load::SizedTexture { id: tex.id(), size };
                                let resp = ui
                                    .add(egui::Image::new(sized).sense(egui::Sense::click()))
                                    .on_hover_text("Click to copy")
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                let visuals = ui.visuals();
                                let rounding = egui::CornerRadius::same(6);
                                let stroke = if resp.hovered() {
                                    egui::Stroke::new(1.5, visuals.widgets.hovered.fg_stroke.color)
                                } else {
                                    egui::Stroke::new(
                                        1.0,
                                        visuals
                                            .widgets
                                            .inactive
                                            .fg_stroke
                                            .color
                                            .gamma_multiply(0.25),
                                    )
                                };
                                ui.painter().rect_stroke(
                                    resp.rect.expand(2.0),
                                    rounding,
                                    stroke,
                                    StrokeKind::Inside,
                                );
                                if resp.clicked() {
                                    pending_restore = Some(entry.clone());
                                }
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
                if let Some(pos) = self
                    .history
                    .iter()
                    .position(|e| content_key(&e.content) == key)
                {
                    let mut existing = self.history.remove(pos);
                    existing.ts = now;
                    self.history.push(existing);
                } else {
                    self.history.push(ClipboardEntry {
                        ts: now,
                        content: entry.content.clone(),
                    });
                }
                let _ = append_touch(&key, now);
            } else {
                self.seen.insert(key.clone());
                let new_entry = ClipboardEntry {
                    ts: now,
                    content: entry.content.clone(),
                };
                self.history.push(new_entry.clone());
                let _ = append_put(&key, &new_entry.content, now);
            }
        }
    }
}
