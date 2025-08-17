use std::collections::HashMap;
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::clip::content_key;
use crate::paths::history_path;
use crate::crypto::{derive_save_nonce, encrypt_data_to_file, decrypt_file};
use crate::types::{ClipboardContent, ClipboardEntry, FileModel, Meta};

const AUTOSAVE_OPS_THRESHOLD: usize = 10;

fn meta_path() -> std::path::PathBuf {
    history_path().with_extension("meta.json")
}

fn load_meta() -> Result<Meta> {
    let p = meta_path();
    if !p.exists() {
        return Ok(Meta { version: 1, next_counter: 1 });
    }
    let bytes = std::fs::read(p)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn store_meta(m: &Meta) -> Result<()> {
    let p = meta_path();
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(m)?)?;
    std::fs::rename(tmp, p)?;
    Ok(())
}

pub struct Store {
    // Crypto params
    key: [u8; 32],
    base_nonce: [u8; 24],

    // Data
    entries: Vec<ClipboardEntry>,
    index: HashMap<String, usize>,

    // Persistence state
    next_counter: u64,
    ops_since_save: usize,
    dirty: bool,
}

impl Store {
    pub fn open_or_create(key: [u8; 32], base_nonce: [u8; 24]) -> Result<Self> {
        let path = history_path();
        let mut meta = load_meta()?;

        let (entries, index) = if path.exists() {
            let try_with = |ctr: u64| -> Result<Vec<ClipboardEntry>> {
                let nonce = derive_save_nonce(&key, &base_nonce, ctr);
                let (res, bytes) = decrypt_file(
                    path.to_str().unwrap(),
                    &key,
                    &nonce,
                );
                res?;
                let model: FileModel = serde_json::from_slice(&bytes)?;
                Ok(model.entries)
            };

            let entries = if meta.next_counter > 0 {
                if let Ok(v) = try_with(meta.next_counter - 1) {
                    v
                } else {
                    try_with(meta.next_counter)?
                }
            } else {
                Vec::new()
            };

            let mut index = HashMap::new();
            for (i, e) in entries.iter().enumerate() {
                index.insert(content_key(&e.content), i);
            }
            (entries, index)
        } else {
            meta.next_counter = 1;
            (Vec::new(), HashMap::new())
        };

        store_meta(&meta)?;

        Ok(Self {
            key,
            base_nonce,
            entries,
            index,
            next_counter: meta.next_counter,
            ops_since_save: 0,
            dirty: false,
        })
    }


    pub fn entries(&self) -> &Vec<ClipboardEntry> { &self.entries }

    pub fn put(&mut self, ts: DateTime<Utc>, content: ClipboardContent) {
        let k = content_key(&content);
        if let Some(&i) = self.index.get(&k) {
            self.entries[i].ts = ts;
            let e = self.entries.remove(i);
            self.entries.push(e);
            self.rebuild_index();
        } else {
            self.entries.push(ClipboardEntry { ts, content });
            self.index.insert(k, self.entries.len() - 1);
        }
        self.mark_dirty();
        let _ = self.autosave_if_needed();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.mark_dirty();
        let _ = self.autosave_if_needed();
    }

    pub fn force_save(&mut self) -> Result<()> {
        if !self.dirty { return Ok(()); }

        let path = history_path();
        let tmp_enc = path.with_extension("json.tmp"); // write-then-rename

        let model = FileModel { version: 1, entries: self.entries.clone() };
        let json = serde_json::to_vec(&model)?;
        let nonce = derive_save_nonce(&self.key, &self.base_nonce, self.next_counter);

        encrypt_data_to_file(&json, tmp_enc.to_str().unwrap(), &self.key, &nonce)?;
        std::fs::rename(&tmp_enc, &path)?;
        self.next_counter = self.next_counter.saturating_add(1);
        store_meta(&Meta { version: 1, next_counter: self.next_counter })?;

        self.dirty = false;
        self.ops_since_save = 0;
        Ok(())
    }

    fn autosave_if_needed(&mut self) -> Result<()> {
        if self.dirty && self.ops_since_save >= AUTOSAVE_OPS_THRESHOLD {
            self.force_save()?;
        }
        Ok(())
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, e) in self.entries.iter().enumerate() {
            self.index.insert(content_key(&e.content), i);
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.ops_since_save += 1;
    }
}
