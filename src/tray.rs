
use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent as TrayMenuEvent, MenuId, MenuItem},
};

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
                #[cfg(target_os = "linux")]{
                    gtk::init().expect("gtk::init failed");
                }

                let menu = Menu::new();
                let open = MenuItem::new("Open", true, None);
                let quit = MenuItem::new("Quit", true, None);
                menu.append(&open).unwrap();
                menu.append(&quit).unwrap();

                let icon = load_icon_png(include_bytes!("../img/tray.png")).unwrap();

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

                #[cfg(target_os = "linux")]{
                    gtk::main();
                }
                
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

            let icon = load_icon_png(include_bytes!("../img/tray.png"))
                .unwrap_or_else(|_| {
                    anyhow::bail!("Failed to load tray icon. Ensure the image is a valid PNG.");
                });

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

fn load_icon_png(bytes: &[u8]) -> anyhow::Result<Icon> {
    let img = image::load_from_memory(bytes)?.into_rgba8();
    let (w, h) = img.dimensions();
    Ok(Icon::from_rgba(img.into_raw(), w, h)?)
}
