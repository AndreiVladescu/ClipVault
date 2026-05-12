use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod base64_serde {
    use base64::{Engine as _, engine::general_purpose};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    // Keep "ImageBase64" tag for backward compat with v1 history files
    #[serde(rename = "ImageBase64")]
    Image(#[serde(with = "base64_serde")] Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub ts: DateTime<Utc>,
    pub content: ClipboardContent,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(dead_code)]
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
