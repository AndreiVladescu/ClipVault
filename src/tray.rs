use crate::assets::ICON_TRAY;

use egui::Context;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent as TrayMenuEvent, MenuId, MenuItem},
};

#[cfg(target_os = "windows")]
use tray_icon::{MouseButton, TrayIcon, TrayIconEvent};

pub struct Tray {
    #[cfg(not(target_os = "linux"))]
    _icon: TrayIcon,
    pub open_id: MenuId,
    pub quit_id: MenuId,
    // On Linux: event thread converts menu events here so try_recv() is cheap.
    // On non-Linux: None, try_recv() falls back to polling the static receivers.
    event_rx: Option<crossbeam::channel::Receiver<TrayEvent>>,
}

pub enum TrayEvent {
    OpenRequested,
    QuitRequested,
    None,
}

impl Tray {
    pub fn new(wake: Arc<OnceCell<Context>>) -> anyhow::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            use std::sync::mpsc;
            let (tx_ids, rx_ids) = mpsc::sync_channel::<(MenuId, MenuId)>(1);
            let (event_tx, event_rx) = crossbeam::channel::unbounded::<TrayEvent>();

            std::thread::spawn(move || {
                gtk::init().expect("gtk::init failed");

                let menu = Menu::new();
                let open = MenuItem::new("Open", true, None);
                let quit = MenuItem::new("Quit", true, None);
                menu.append(&open).unwrap();
                menu.append(&quit).unwrap();

                let icon = crate::assets::get_bytes(ICON_TRAY)
                    .ok_or_else(|| anyhow::anyhow!("missing embedded app icon"))
                    .unwrap();
                let icon = crate::assets::tray_icon_from_png(&icon).unwrap();

                let _tray_icon = TrayIconBuilder::new()
                    .with_tooltip("ClipVault")
                    .with_menu(Box::new(menu))
                    .with_icon(icon)
                    .build()
                    .expect("tray build");

                tx_ids
                    .send((open.id().to_owned(), quit.id().to_owned()))
                    .ok();

                gtk::main();
            });

            let (open_id, quit_id) = rx_ids.recv()?;

            // Blocking watcher: converts TrayMenuEvent → TrayEvent and wakes egui.
            // Uses blocking recv so it sleeps instead of spinning.
            let open_id_w = open_id.clone();
            let quit_id_w = quit_id.clone();
            std::thread::spawn(move || {
                loop {
                    match TrayMenuEvent::receiver().recv() {
                        Ok(ev) => {
                            let event = if ev.id == open_id_w {
                                TrayEvent::OpenRequested
                            } else if ev.id == quit_id_w {
                                TrayEvent::QuitRequested
                            } else {
                                continue;
                            };
                            let _ = event_tx.send(event);
                            if let Some(ctx) = wake.get() {
                                ctx.request_repaint();
                            }
                        }
                        Err(_) => break, // sender dropped
                    }
                }
            });

            Ok(Self {
                open_id,
                quit_id,
                event_rx: Some(event_rx),
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            let menu = Menu::new();
            let open = MenuItem::new("Open", true, None);
            let quit = MenuItem::new("Quit", true, None);
            menu.append(&open)?;
            menu.append(&quit)?;

            let icon = crate::assets::get_bytes(ICON_TRAY)
                .ok_or_else(|| anyhow::anyhow!("missing embedded app icon"))
                .unwrap();
            let icon = crate::assets::tray_icon_from_png(&icon).unwrap();

            let tray_icon = TrayIconBuilder::new()
                .with_tooltip("ClipVault")
                .with_menu(Box::new(menu))
                .with_icon(icon)
                .build()?;

            Ok(Self {
                open_id: open.id().to_owned(),
                quit_id: quit.id().to_owned(),
                _icon: tray_icon,
                event_rx: None,
            })
        }
    }

    pub fn try_recv(&self) -> TrayEvent {
        // Linux: drain the internal event channel populated by the watcher thread.
        if let Some(ref rx) = self.event_rx {
            return rx.try_recv().unwrap_or(TrayEvent::None);
        }

        // Non-Linux fallback: poll the static receivers directly.
        #[cfg(not(target_os = "linux"))]
        if let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            match ev {
                TrayIconEvent::Click { button, .. } | TrayIconEvent::DoubleClick { button, .. } => {
                    if button == MouseButton::Left {
                        return TrayEvent::OpenRequested;
                    }
                }
                _ => {}
            }
        }

        if let Ok(ev) = TrayMenuEvent::receiver().try_recv() {
            if ev.id == self.open_id {
                return TrayEvent::OpenRequested;
            } else if ev.id == self.quit_id {
                return TrayEvent::QuitRequested;
            }
        }

        TrayEvent::None
    }
}
