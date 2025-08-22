use crate::assets::{ICON_SETTINGS, load_texture_from_asset};
use crate::clip::{content_key, set_clipboard};
use crate::crypto::{decrypt_file, derivate_crypto_params, derive_save_nonce};
use crate::img::base64_to_imagedata;
use crate::paths::history_path;
use crate::storage::Store;
use crate::tray;
use crate::tray::TrayEvent;
use crate::types::{ClipboardContent, ClipboardEntry, HotkeyMsg, Meta, UnlockResult};

use anyhow::anyhow;
use chrono::Utc;
use crossbeam::channel::Receiver;
use egui::{RichText, StrokeKind};
use std::collections::HashMap;

pub struct ClipAppLocked {
    passphrase: String,
    key: [u8; 32],
    nonce: [u8; 24],
    loaded_crypto_params: bool,
    create_mode: bool,

    outcome_tx: Option<crossbeam::channel::Sender<UnlockResult>>,
    outcome_sent: bool,
}

impl ClipAppLocked {
    pub fn new(outcome_tx: crossbeam::channel::Sender<UnlockResult>) -> Self {
        Self {
            passphrase: String::new(),
            key: [0; 32],
            nonce: [0; 24],
            loaded_crypto_params: false,
            create_mode: !history_path().exists(),
            outcome_tx: Some(outcome_tx),
            outcome_sent: false,
        }
    }

    pub fn set_crypto_params(&mut self) {
        let (key, nonce) = derivate_crypto_params(self.passphrase.clone());
        self.key = key;
        self.nonce = nonce;
        self.loaded_crypto_params = true;
    }

    pub fn try_decrypt_history(&self) -> anyhow::Result<()> {
        let path = history_path();
        let meta_path = path.with_extension("meta.json");

        // helper to attempt a decrypt with a given nonce
        let try_nonce = |nonce: [u8; 24]| -> anyhow::Result<()> {
            decrypt_file(path.to_str().unwrap(), &self.key, &nonce).0
        };

        if meta_path.exists() {
            // Read sidecar meta to learn the save counter
            let bytes = std::fs::read(&meta_path)?;
            let meta: Meta = serde_json::from_slice(&bytes)?;
            if meta.next_counter > 0 {
                // Most recent successful save used counter = next_counter - 1
                let c_prev = meta.next_counter.saturating_sub(1);
                let n_prev = derive_save_nonce(&self.key, &self.nonce, c_prev);
                if try_nonce(n_prev).is_ok() {
                    return Ok(());
                }
                // If meta advanced before data finished writing, try current next_counter
                let n_curr = derive_save_nonce(&self.key, &self.nonce, meta.next_counter);
                if try_nonce(n_curr).is_ok() {
                    return Ok(());
                }
            }
            // Fallback for very old files that used the base nonce directly
            try_nonce(self.nonce)
                .map_err(|_| anyhow!("Decryption failed with derived and base nonces"))
        } else {
            // If no legacy file; try base nonce, then first derived counter = 1
            if try_nonce(self.nonce).is_ok() {
                return Ok(());
            }
            let n1 = derive_save_nonce(&self.key, &self.nonce, 1);
            try_nonce(n1).map_err(|_| anyhow!("Decryption failed (no meta present)"))
        }
    }
    fn passphrase_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut submit = false;

        ui.horizontal(|ui| {
            let old_spacing = ui.spacing().clone();
            ui.spacing_mut().item_spacing.x = 4.0;

            let h = ui.spacing().interact_size.y;
            let eye_resp = ui
                .add_sized([h, h], egui::Button::new(egui::RichText::new("👁")))
                .on_hover_text("Hold to show");
            let held = eye_resp.is_pointer_button_down_on();

            let field_resp = ui.add(
                egui::TextEdit::singleline(&mut self.passphrase)
                    .password(!held)
                    .desired_width(f32::INFINITY),
            );
            if field_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }

            *ui.spacing_mut() = old_spacing;
        });

        let btn_text = if self.create_mode { "Create" } else { "Unlock" };
        let pass_btn =
            egui::Button::new(egui::RichText::new(btn_text).size(16.0)).corner_radius(6.0);
        if ui
            .add_sized(
                [ui.available_width(), ui.spacing().interact_size.y],
                pass_btn,
            )
            .clicked()
        {
            submit = true;
        }

        submit
    }

    fn handle_submit(&mut self, ctx: &egui::Context) {
        if self.passphrase.is_empty() {
            println!("Passphrase cannot be empty.");
            return;
        }
        self.set_crypto_params();
        if self.create_mode {
            if let Some(tx) = self.outcome_tx.take() {
                let _ = tx.send(UnlockResult::Unlocked {
                    key: self.key,
                    nonce: self.nonce,
                });
                self.outcome_sent = true;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            if self.try_decrypt_history().is_ok() {
                if let Some(tx) = self.outcome_tx.take() {
                    let _ = tx.send(UnlockResult::Unlocked {
                        key: self.key,
                        nonce: self.nonce,
                    });
                    self.outcome_sent = true;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                println!("Failed to decrypt history with the provided passphrase.");
            }
        }
    }
}

impl eframe::App for ClipAppLocked {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let msg_locked = "ClipVault is locked.\n\nTo unlock you need to enter the passphrase.";
            let msg_create = "ClipVault is not initialized.\n\nSet passphrase first.";
            ui.label(
                RichText::new(if self.create_mode {
                    msg_create
                } else {
                    msg_locked
                })
                .size(14.0),
            );
            ui.separator();

            if self.passphrase_ui(ui) {
                self.handle_submit(ctx);
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if !self.outcome_sent {
            if let Some(tx) = self.outcome_tx.take() {
                let _ = tx.send(UnlockResult::Cancelled);
            }
        }

        self.key = [0; 32];
        self.nonce = [0; 24];
        self.passphrase.clear();
    }
}

pub struct ClipApp {
    tray: std::sync::Arc<tray::Tray>,
    rx: crossbeam::channel::Receiver<ClipboardEntry>,
    store: Store,
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
        store: Store,
        hotkey_rx: Receiver<HotkeyMsg>,
    ) -> Self {
        Self {
            tray,
            rx,
            store,
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
                self.store.force_save().ok();
                return;
            }
            TrayEvent::None => {}
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        while let Ok(entry) = self.rx.try_recv() {
            self.store.put(entry.ts, entry.content.clone());
        }

        // Top panel
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("ClipVault").size(24.0));
                ui.separator();
                ui.label(egui::RichText::new("Filter").size(18.0));
                ui.text_edit_singleline(&mut self.filter);
                if let Some(tex) = load_texture_from_asset(ctx, ICON_SETTINGS) {
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
                    if ui.button("Save now").clicked() {
                        if let Err(e) = self.store.force_save() {
                            eprintln!("Save failed: {e}");
                        }
                    }
                    if ui.button("Clear history").clicked() {
                        self.store.clear();
                        let _ = self.store.force_save();
                    }
                });
        }

        let mut pending_restore: Option<ClipboardEntry> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(300.0);
                });

                let items = self.store.entries();
                let q: String = self.filter.to_lowercase();
                for idx in (0..items.len()).rev() {
                    let entry: ClipboardEntry = items[idx].clone();

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
            let now = Utc::now();
            self.store.put(now, entry.content.clone());
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.store.force_save();
    }
}
