#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod assets;
mod clip;
mod crypto;
mod img;
mod paths;
mod singleton;
mod storage;
mod tray;
mod types;
mod ui;

use crate::assets::{ICON_TRAY, get_bytes, icon_data_from_png};
use crate::clip::{clipboard_entry_hash, spawn_watcher};
use crate::singleton::setup_single_instance;
use crate::storage::Store;
use crate::types::{HotkeyMsg, UnlockResult};

use crossbeam::channel;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};

use std::time::{Duration, Instant};

fn unencrypted_main(key: [u8; 32], nonce: [u8; 24], activate_rx: crossbeam::channel::Receiver<()>) -> anyhow::Result<()> {
    let (hk_tx, hk_rx) = channel::unbounded::<HotkeyMsg>();
    std::thread::spawn(move || {
        let mgr = GlobalHotKeyManager::new().expect("hotkey manager");
        let hk = HotKey::new(Some(Modifiers::SUPER), Code::KeyV);
        mgr.register(hk).expect("register hotkey");

        let rx = GlobalHotKeyEvent::receiver();

        // Debounce
        let mut last = Instant::now() - Duration::from_millis(500);

        loop {
            if let Ok(ev) = rx.recv() {
                if ev.state == HotKeyState::Pressed && last.elapsed() > Duration::from_millis(250) {
                    let _ = hk_tx.send(HotkeyMsg::ToggleWindow);
                    last = Instant::now();
                }
            }
        }
    });

    let store = Store::open_or_create(key, nonce)?;
    let last_hash = store
        .entries()
        .last()
        .map(|e| clipboard_entry_hash(&e.content));

    let (tx, rx) = crossbeam::channel::unbounded();
    spawn_watcher(tx, last_hash);

    let icon = get_bytes(ICON_TRAY)
        .and_then(|b| icon_data_from_png(&b))
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([512.0, 600.0])
            .with_resizable(false)
            .with_decorations(false)
            .with_visible(false)
            .with_icon(icon),
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    let tray = std::sync::Arc::new(tray::Tray::new()?);
    let tray_clone = tray.clone();

    let res = eframe::run_native(
        "ClipVault",
        options,
        Box::new(move |_cc| {
            Ok::<Box<dyn eframe::App>, _>(Box::new(ui::ClipApp::new(tray_clone, rx, store, hk_rx, activate_rx)))
        }),
    );

    if let Err(e) = res {
        eprintln!("eframe error: {e}");
    }
    Ok(())
}

fn encrypted_main() -> anyhow::Result<([u8; 32], [u8; 24])> {
    let (tx, rx) = channel::bounded::<UnlockResult>(1);

    let icon = get_bytes(ICON_TRAY)
        .and_then(|b| icon_data_from_png(&b))
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 118.0])
            .with_resizable(false)
            .with_visible(true)
            .with_icon(icon),
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    let res = eframe::run_native(
        "ClipVault",
        options,
        Box::new(move |_cc| Ok::<Box<dyn eframe::App>, _>(Box::new(ui::ClipAppLocked::new(tx)))),
    );

    if let Err(e) = res {
        eprintln!("eframe error: {e}");
    }

    let outcome = rx
        .recv_timeout(Duration::from_millis(50))
        .unwrap_or(UnlockResult::Cancelled);

    match outcome {
        UnlockResult::Unlocked { key, nonce } => {
            return Ok((key, nonce));
        }
        UnlockResult::Cancelled => {
            return Err(anyhow::anyhow!("Failed to unlock ClipVault: {outcome:?}"));
        }
    }
}

fn main() -> anyhow::Result<()> {
    let (activate_tx, activate_rx) = crossbeam::channel::unbounded();
    if !setup_single_instance(activate_tx.clone()) {
        return Ok(());
    }

    let crypto_params = encrypted_main();
    match crypto_params {
        Ok((key, nonce)) => {
            if let Err(e) = unencrypted_main(key, nonce, activate_rx) {
                eprintln!("Error in unencrypted main: {e}");
                return Err(e);
            }
        }
        Err(e) => {
            return anyhow::Result::Err(anyhow::anyhow!("Failed to decrypt history: {e}"));
        }
    }

    return Ok(());
}
