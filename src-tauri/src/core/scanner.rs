use crate::core::cache::{file_mtime, path_string, CacheIndex};
use crate::core::matcher::NameMatcher;
use crate::core::types::{CacheEntry, IocEntry, MatchedPhoto, ScanStats};
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
    let roots = normalized_scan_roots(roots);

    let walker = roots
        .iter()
        .flat_map(|root| WalkDir::new(root).follow_links(false).into_iter());

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
    let mut matched_species = HashSet::new();
    let mut seen_paths = HashSet::new();

    for item in results {
        if !seen_paths.insert(item.cache_entry.path.clone()) {
            continue;
        }
        total_files += 1;
        cache_entries.push(item.cache_entry);
        if let Some(photo) = item.matched_photo {
            matched_files += 1;
            matched_species.insert(photo.species_idx);
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
            matched_species: matched_species.len(),
            unmatched_files,
        },
    }
}

fn normalized_scan_roots(roots: &[String]) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = roots
        .iter()
        .map(PathBuf::from)
        .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
        .collect();

    candidates.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });

    let mut unique = Vec::<PathBuf>::new();
    for candidate in candidates {
        if unique.iter().any(|root| candidate.starts_with(root)) {
            continue;
        }
        unique.push(candidate);
    }
    unique
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cache::CacheIndex;

    #[test]
    fn overlapping_roots_do_not_count_the_same_photo_twice() {
        let base = std::env::temp_dir().join(format!(
            "birdindex2-scanner-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = base.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("白头鹎.jpg"), b"test").unwrap();

        let entries = vec![IocEntry {
            order: "PASSERIFORMES".into(),
            family: "Pycnonotidae".into(),
            latin: "Pycnonotus sinensis".into(),
            chinese: "白头鹎".into(),
        }];
        let latin_index = HashMap::from([("pycnonotus sinensis".into(), 0)]);
        let matcher = NameMatcher::new(&entries);
        let roots = vec![
            base.to_string_lossy().to_string(),
            nested.to_string_lossy().to_string(),
        ];

        assert_eq!(normalized_scan_roots(&roots).len(), 1);

        let output = scan_paths(
            &roots,
            &entries,
            &latin_index,
            &matcher,
            &CacheIndex::empty(),
        );

        assert_eq!(output.stats.total_files, 1);
        assert_eq!(output.stats.matched_files, 1);
        assert_eq!(output.stats.matched_species, 1);
        assert_eq!(output.matches.len(), 1);

        std::fs::remove_dir_all(base).unwrap();
    }
}
