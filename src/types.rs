use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    ImageBase64(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub ts: DateTime<Utc>,
    pub content: ClipboardContent,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LogRec {
    Put {
        key: String,
        ts: DateTime<Utc>,
        content: ClipboardContent,
    },
    Touch {
        key: String,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
pub enum HotkeyMsg {
    ToggleWindow,
}

#[derive(Debug)]
pub enum UnlockResult {
    Unlocked { key: [u8; 32], nonce: [u8; 24] },
    Cancelled,
}

#[derive(Serialize, Deserialize)]
pub struct FileModel {
    pub version: u8,
    pub entries: Vec<ClipboardEntry>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub struct Meta {
    pub version: u8,
    pub next_counter: u64,
}
