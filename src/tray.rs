use crate::assets::ICON_TRAY;

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
}

pub enum TrayEvent {
    OpenRequested,
    QuitRequested,
    None,
}
impl Tray {
    pub fn new() -> anyhow::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            use std::sync::mpsc;
            let (tx_ids, rx_ids) = mpsc::sync_channel::<(MenuId, MenuId)>(1);

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

                // Send IDs back so main thread can match MenuEvent ids.
                tx_ids
                    .send((open.id().to_owned(), quit.id().to_owned()))
                    .ok();

                gtk::main();
            });

            let (open_id, quit_id) = rx_ids.recv()?;
            Ok(Self { open_id, quit_id })
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
            })
        }
    }

    pub fn try_recv(&self) -> TrayEvent {
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
