mod types;
mod storage;
mod img;
mod clip;
mod ui;

use crate::clip::{clipboard_entry_hash, spawn_watcher};
use crate::storage::{compact_history_log, load_history_mru};
use crate::types::ClipboardEntry;

use std::collections::HashSet;

fn main() -> anyhow::Result<()> {
    if let Err(e) = compact_history_log() {
        eprintln!("Compaction failed: {e}");
    }

    let history: Vec<ClipboardEntry> = load_history_mru()?;
    let last_hash = history.last().map(|e| clipboard_entry_hash(&e.content));
    let seen: HashSet<String> =
        history.iter().map(|e| clip::content_key(&e.content)).collect();

    let (tx, rx) = crossbeam::channel::unbounded();
    spawn_watcher(tx, last_hash);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([512.0, 600.0])
            .with_resizable(false)
            .with_decorations(false),
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    let res = eframe::run_native(
        "ClipVault",
        options,
        Box::new(move |_cc| {
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(
                Box::new(ui::ClipApp::new(rx, history, seen))
            )
        }),
    );

    if let Err(e) = res { eprintln!("eframe error: {e}"); }
    Ok(())
}
