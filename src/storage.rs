use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    collections::HashMap
};

use chrono::{DateTime, Utc};
use serde_json;

use crate::types::{Agg, ClipboardContent, ClipboardEntry, LogRec};
use crate::paths::history_path;

pub fn compact_history_log() -> anyhow::Result<()> {
    let path = history_path();
    if !path.exists() { return Ok(()); }

    let file: std::fs::File = OpenOptions::new().read(true).open(&path)?;
    let reader: BufReader<std::fs::File> = BufReader::new(file);
    let mut map: HashMap<String, Agg> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let rec: LogRec = serde_json::from_str(&line)?;
        match rec {
            LogRec::Put { key, ts, content } => {
                map.entry(key).and_modify(|a| {
                    if ts < a.created_ts { a.created_ts = ts; }
                    if ts > a.last_ts    { a.last_ts    = ts; }
                }).or_insert(Agg { content, created_ts: ts, last_ts: ts });
            }
            LogRec::Touch { key, ts } => {
                if let Some(a) = map.get_mut(&key) {
                    if ts > a.last_ts { a.last_ts = ts; }
                }
            }
        }
    }

    let tmp = path.with_extension("tmp");
    {
        let mut out = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&tmp)?;
        let mut items: Vec<_> = map.into_iter().collect();
        items.sort_by(|a,b| b.1.last_ts.cmp(&a.1.last_ts));

        for (key, agg) in items {
            let put: LogRec = LogRec::Put { key: key.clone(), ts: agg.created_ts, content: agg.content.clone() };
            serde_json::to_writer(&mut out, &put)?; out.write_all(b"\n")?;
            if agg.last_ts > agg.created_ts {
                let touch: LogRec = LogRec::Touch { key, ts: agg.last_ts };
                serde_json::to_writer(&mut out, &touch)?; out.write_all(b"\n")?;
            }
        }
        out.flush()?;
    }
    let _ = std::fs::remove_file(path);
    let path = history_path();
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn append_put(key: &str, content: &ClipboardContent, ts: DateTime<Utc>) -> anyhow::Result<()> {
    let path = history_path();
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut f, &LogRec::Put { key: key.to_string(), ts, content: content.clone() })?;
    f.write_all(b"\n")?;
    Ok(())
}

pub fn append_touch(key: &str, ts: DateTime<Utc>) -> anyhow::Result<()> {
    let path = history_path();
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut f, &LogRec::Touch { key: key.to_string(), ts })?;
    f.write_all(b"\n")?;
    Ok(())
}

pub fn load_history_mru() -> anyhow::Result<Vec<ClipboardEntry>> {
    let path = history_path();
    let file = match OpenOptions::new().read(true).open(path) {
        Ok(f) => f, Err(_) => return Ok(Vec::new()),
    };
    let reader: BufReader<std::fs::File> = BufReader::new(file);

    let mut map: HashMap<String, (ClipboardContent, DateTime<Utc>)> = HashMap::new();

    for line in reader.lines() {
        let line: String = line?;
        if line.trim().is_empty() { continue; }
        let rec: LogRec = serde_json::from_str(&line)?;
        match rec {
            LogRec::Put { key, ts, content } => {
                let e: &mut (ClipboardContent, DateTime<Utc>) = map.entry(key).or_insert((content, ts));
                if ts > e.1 { e.1 = ts; }
            }
            LogRec::Touch { key, ts } => {
                if let Some(e) = map.get_mut(&key) {
                    e.1 = ts;
                }
            }
        }
    }

    let mut v: Vec<ClipboardEntry> = map
        .into_iter()
        .map(|(_k, (content, ts))| ClipboardEntry { ts, content })
        .collect();
    v.sort_by(|a: &ClipboardEntry, b: &ClipboardEntry| b.ts.cmp(&a.ts));
    Ok(v)
}
