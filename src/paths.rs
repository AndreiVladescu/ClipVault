use std::path::PathBuf;

pub fn app_config_dir() -> PathBuf {
    let mut dir = dirs_next::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    dir.push("ClipVault");
    dir
}

pub fn history_path() -> PathBuf {
    if let Ok(p) = std::env::var("CLIPVAULT_HISTORY") {
        let path = PathBuf::from(p);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(parent);
        }
        return path;
    }

    let dir = app_config_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join(".clipvault_clipboard.json")
}
