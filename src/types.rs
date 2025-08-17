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

#[derive(Clone)]
pub struct Agg {
    pub content: ClipboardContent,
    pub created_ts: DateTime<Utc>,
    pub last_ts: DateTime<Utc>,
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
