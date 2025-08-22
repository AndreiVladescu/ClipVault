use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver, Sender};
use tray_icon::{
    MouseButton,
    TrayIconBuilder,
    TrayIconEvent,
    menu::{Menu, MenuEvent as TrayMenuEvent, MenuId, MenuItem},
};
use std::thread::JoinHandle;

use crate::assets::ICON_TRAY;

pub enum TrayEvent {
    OpenRequested,
    QuitRequested,
    None,
}

pub struct Tray {
    rx: Receiver<TrayEvent>,
    #[cfg(windows)]
    _thread: JoinHandle<()>,
    #[cfg(target_os = "linux")]
    _thread: JoinHandle<()>,
}

impl Tray {
    pub fn new(egui_ctx: egui::Context) -> Result<Self> {
        let (tx, rx) = unbounded::<TrayEvent>();

        #[cfg(windows)]
        let handle = spawn_tray_windows(egui_ctx.clone(), tx.clone())?;

        #[cfg(target_os = "linux")]
        let handle = spawn_tray_linux(egui_ctx.clone(), tx.clone())?;

        Ok(Self {
            rx,
            #[cfg(any(windows, target_os = "linux"))]
            _thread: handle,
        })
    }

    pub fn try_recv(&self) -> TrayEvent {
        self.rx.try_recv().unwrap_or(TrayEvent::None)
    }
}

#[cfg(windows)]
fn spawn_tray_windows(
    egui_ctx: egui::Context,
    tx: Sender<TrayEvent>,
) -> Result<JoinHandle<()>> {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    let handle = std::thread::spawn(move || {
        let menu = Menu::new();
        let open = MenuItem::new("Open", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append(&open).unwrap();
        menu.append(&quit).unwrap();

        let icon_bytes = crate::assets::get_bytes(ICON_TRAY)
            .expect("missing embedded app icon");
        let icon = crate::assets::tray_icon_from_png(&icon_bytes).expect("invalid icon");

        let _tray = TrayIconBuilder::new()
            .with_tooltip("ClipVault")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .expect("tray build");
        {
            let tx = tx.clone();
            let egui_ctx = egui_ctx.clone();
            TrayIconEvent::set_event_handler(Some({
                let tx = tx.clone();
                let egui_ctx = egui_ctx.clone();
                move |e: tray_icon::TrayIconEvent| {
                    use tray_icon::TrayIconEvent::*;
                    match e {
                        Click { button, .. } | DoubleClick { button, .. } => {
                            if button == MouseButton::Left {
                                egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                                egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                                let _ = tx.send(TrayEvent::OpenRequested);
                            }
                        }
                        _ => {}
                    }
                    egui_ctx.request_repaint();
                }
            }));
        }
        {
            let tx = tx.clone();
            let egui_ctx = egui_ctx.clone();
            let open_id: MenuId = open.id().clone();
            let quit_id: MenuId = quit.id().clone();
            TrayMenuEvent::set_event_handler(Some(move |e: TrayMenuEvent| {
                if e.id == open_id {
                    let _ = tx.send(TrayEvent::OpenRequested);
                } else if e.id == quit_id {
                    let _ = tx.send(TrayEvent::QuitRequested);
                }
                egui_ctx.request_repaint();
            }));
        }

        // Windows message loop
        unsafe {
            let mut msg = MSG::default();
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if r.0 == 0 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    Ok(handle)
}

#[cfg(target_os = "linux")]
fn spawn_tray_linux(
    egui_ctx: egui::Context,
    tx: Sender<TrayEvent>,
) -> Result<JoinHandle<()>> {
    let handle = std::thread::spawn(move || {
        gtk::init().expect("gtk::init failed");

        let menu = Menu::new();
        let open = MenuItem::new("Open", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append(&open).unwrap();
        menu.append(&quit).unwrap();

        let icon_bytes = crate::assets::get_bytes(ICON_TRAY)
            .expect("missing embedded app icon");
        let icon = crate::assets::tray_icon_from_png(&icon_bytes).expect("invalid icon");

        let _tray_icon = TrayIconBuilder::new()
            .with_tooltip("ClipVault")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .expect("tray build");
        {
            let tx = tx.clone();
            let egui_ctx = egui_ctx.clone();
            TrayIconEvent::set_event_handler(Some({
                let tx = tx.clone();
                let egui_ctx = egui_ctx.clone();
                move |e: tray_icon::TrayIconEvent| {
                    use tray_icon::TrayIconEvent::*;
                    match e {
                        Click { button, .. } | DoubleClick { button, .. } => {
                            if button == MouseButton::Left {
                                egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                                egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                                let _ = tx.send(TrayEvent::OpenRequested);
                            }
                        }
                        _ => {}
                    }
                    egui_ctx.request_repaint();
                }
            }));
        }
        {
            let tx = tx.clone();
            let egui_ctx = egui_ctx.clone();
            let open_id: MenuId = open.id();
            let quit_id: MenuId = quit.id();
            TrayMenuEvent::set_event_handler(Some(move |e: TrayMenuEvent| {
                if e.id == open_id {
                    let _ = tx.send(TrayEvent::OpenRequested);
                } else if e.id == quit_id {
                    let _ = tx.send(TrayEvent::QuitRequested);
                }
                egui_ctx.request_repaint();
            }));
        }
        gtk::main();
    });

    Ok(handle)
}