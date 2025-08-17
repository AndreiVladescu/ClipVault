use std::path::PathBuf;

pub fn history_path() -> PathBuf {
    if let Ok(p) = std::env::var("CLIPVAULT_HISTORY") {
        return PathBuf::from(p);
    }

    let home =
        home::home_dir().unwrap_or_else(|| std::env::current_dir().expect("no home, no cwd?"));
    home.join(".clipvault_clipboard.json")
}
