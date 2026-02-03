use crate::core::types::{CacheEntry, CacheFile};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct CacheIndex {
    pub entries: HashMap<String, CacheEntry>,
}

impl CacheIndex {
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, path: &str) -> Option<&CacheEntry> {
        self.entries.get(path)
    }
}

pub fn load_cache<P: AsRef<Path>>(path: P, ioc_fingerprint: &str) -> Result<CacheIndex> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(CacheIndex::empty());
    }

    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read cache file: {}", path.display()))?;
    let cache: CacheFile = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse cache file: {}", path.display()))?;

    if cache.ioc_fingerprint != ioc_fingerprint {
        return Ok(CacheIndex::empty());
    }

    let mut entries = HashMap::with_capacity(cache.entries.len());
    for entry in cache.entries {
        entries.insert(entry.path.clone(), entry);
    }

    Ok(CacheIndex { entries })
}

pub fn save_cache<P: AsRef<Path>>(
    path: P,
    ioc_fingerprint: &str,
    entries: Vec<CacheEntry>,
) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache dir: {}", parent.display()))?;
    }

    let cache = CacheFile {
        version: 1,
        ioc_fingerprint: ioc_fingerprint.to_string(),
        entries,
    };

    let json = serde_json::to_string_pretty(&cache)
        .with_context(|| format!("Failed to serialize cache file: {}", path.display()))?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write cache file: {}", path.display()))?;
    Ok(())
}

pub fn fingerprint<P: AsRef<Path>>(path: P) -> Result<String> {
    let meta = fs::metadata(&path)
        .with_context(|| format!("Failed to read metadata: {}", path.as_ref().display()))?;
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let mtime = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(json!({
        "size": meta.len(),
        "mtime": mtime,
    })
    .to_string())
}

pub fn file_mtime(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs() as i64)
        .unwrap_or(0)
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
