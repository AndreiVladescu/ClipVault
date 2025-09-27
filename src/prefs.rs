use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Prefs {
    pub auto_launch: bool,
}

fn cfg_app_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(x).join("ClipVault"))
    } else {
        dirs_next::home_dir().map(|h| h.join(".config/ClipVault"))
    }
}

fn cfg_autostart_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(x).join("autostart"))
    } else {
        dirs_next::home_dir().map(|h| h.join(".config/autostart"))
    }
}

fn prefs_path() -> Result<PathBuf> {
    let dir = cfg_app_dir().ok_or_else(|| anyhow::anyhow!("No home/config dir"))?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("prefs.json"))
}

pub fn load() -> Prefs {
    let path = match prefs_path() {
        Ok(p) => p,
        Err(_) => return Prefs::default(),
    };
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Prefs::default(),
    }
}

pub fn save(p: &Prefs) -> Result<()> {
    let path = prefs_path()?;
    let s = serde_json::to_string_pretty(p)?;
    fs::write(path, s)?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn set_autostart(enabled: bool) -> Result<()> {
    let dir = cfg_autostart_dir().ok_or_else(|| anyhow::anyhow!("No autostart dir"))?;
    fs::create_dir_all(&dir)?;
    let desktop = dir.join("clipvault.desktop");

    if enabled {
        let exe = std::env::current_exe()?;
        let exec_line = format!("{}", exe.display());
        let content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=ClipVault\n\
            X-KDE-autostart-after=panel\n\
            X-KDE-StartupNotify=false\n\
            X-KDE-UniqueApplet=true\n\
            X-GNOME-Autostart-Delay=3\n\
            OnlyShowIn=GNOME;KDE;XFCE;LXQt;Unity;\n\
            GenericName=Clipboard Manager\n\
            Comment=An encrypted clipboard manager\n\
            Terminal=false\n\
            Exec=\"{}\"\n\
            Hidden=false\n\
            X-GNOME-Autostart-enabled=true\n",
            exec_line
        );

        fs::write(desktop, content)?;
    } else {
        let _ = fs::remove_file(desktop);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_autostart(_enabled: bool) -> Result<()> {
    Ok(()) // For now, don't do anything on non-Linux systems
}
