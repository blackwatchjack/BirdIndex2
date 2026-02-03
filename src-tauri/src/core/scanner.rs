use crate::core::cache::{file_mtime, path_string, CacheIndex};
use crate::core::matcher::NameMatcher;
use crate::core::types::{CacheEntry, IocEntry, MatchedPhoto, ScanStats};
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScanOutput {
    pub matches: Vec<MatchedPhoto>,
    pub cache_entries: Vec<CacheEntry>,
    pub stats: ScanStats,
}

pub fn scan_paths(
    roots: &[String],
    entries: &[IocEntry],
    latin_index: &HashMap<String, usize>,
    matcher: &NameMatcher,
    cache: &CacheIndex,
) -> ScanOutput {
    let exts: HashSet<&'static str> = ["jpg", "jpeg", "png", "heic"].into_iter().collect();

    let walker = roots.iter().flat_map(|root| {
        WalkDir::new(root)
            .follow_links(false)
            .into_iter()
    });

    let results: Vec<ScanItem> = walker
        .par_bridge()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return None,
            };

            if !entry.file_type().is_file() {
                return None;
            }

            let path = entry.path();
            if !is_supported(path, &exts) {
                return None;
            }

            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => return None,
            };

            let file_stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| file_name.clone());

            let mtime = file_mtime(path);
            let path_str = path_string(path);

            if let Some(cached) = cache.get(&path_str) {
                if cached.mtime == mtime {
                    if let Some(latin) = &cached.species_latin {
                        if let Some(species_idx) = latin_index.get(&latin.to_lowercase()).copied() {
                            return Some(ScanItem::matched(
                                MatchedPhoto {
                                    path: path_str,
                                    file_name,
                                    species_idx,
                                },
                                CacheEntry {
                                    path: cached.path.clone(),
                                    mtime,
                                    species_latin: Some(latin.clone()),
                                },
                            ));
                        }
                    } else {
                        return Some(ScanItem::unmatched(CacheEntry {
                            path: cached.path.clone(),
                            mtime,
                            species_latin: None,
                        }));
                    }
                }
            }

            let species_idx = matcher.match_name(&file_stem);
            match species_idx {
                Some(idx) => Some(ScanItem::matched(
                    MatchedPhoto {
                        path: path_str.clone(),
                        file_name,
                        species_idx: idx,
                    },
                    CacheEntry {
                        path: path_str,
                        mtime,
                        species_latin: Some(entries[idx].latin.clone()),
                    },
                )),
                None => Some(ScanItem::unmatched(CacheEntry {
                    path: path_str,
                    mtime,
                    species_latin: None,
                })),
            }
        })
        .collect();

    let mut matches = Vec::new();
    let mut cache_entries = Vec::with_capacity(results.len());
    let mut total_files = 0usize;
    let mut matched_files = 0usize;

    for item in results {
        total_files += 1;
        cache_entries.push(item.cache_entry);
        if let Some(photo) = item.matched_photo {
            matched_files += 1;
            matches.push(photo);
        }
    }

    let unmatched_files = total_files.saturating_sub(matched_files);

    ScanOutput {
        matches,
        cache_entries,
        stats: ScanStats {
            total_files,
            matched_files,
            unmatched_files,
        },
    }
}

fn is_supported(path: &Path, exts: &HashSet<&'static str>) -> bool {
    let ext = path.extension().and_then(|ext| ext.to_str());
    match ext {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            exts.contains(ext.as_str())
        }
        None => false,
    }
}

struct ScanItem {
    matched_photo: Option<MatchedPhoto>,
    cache_entry: CacheEntry,
}

impl ScanItem {
    fn matched(matched_photo: MatchedPhoto, cache_entry: CacheEntry) -> Self {
        Self {
            matched_photo: Some(matched_photo),
            cache_entry,
        }
    }

    fn unmatched(cache_entry: CacheEntry) -> Self {
        Self {
            matched_photo: None,
            cache_entry,
        }
    }
}
