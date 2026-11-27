use crate::core::cache::{file_mtime, path_string, CacheIndex};
use crate::core::matcher::NameMatcher;
use crate::core::types::{CacheEntry, IocEntry, MatchedMedia, MediaType, ScanStats};
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScanOutput {
    pub matches: Vec<MatchedMedia>,
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
            let media_type = media_type(path)?;

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
                                MatchedMedia {
                                    path: path_str,
                                    file_name,
                                    media_type,
                                    species_idx,
                                },
                                CacheEntry {
                                    path: cached.path.clone(),
                                    mtime,
                                    species_latin: Some(latin.clone()),
                                    media_type,
                                },
                            ));
                        }
                    } else {
                        return Some(ScanItem::unmatched(CacheEntry {
                            path: cached.path.clone(),
                            mtime,
                            species_latin: None,
                            media_type,
                        }));
                    }
                }
            }

            let species_idx = matcher.match_name(&file_stem);
            match species_idx {
                Some(idx) => Some(ScanItem::matched(
                    MatchedMedia {
                        path: path_str.clone(),
                        file_name,
                        media_type,
                        species_idx: idx,
                    },
                    CacheEntry {
                        path: path_str,
                        mtime,
                        species_latin: Some(entries[idx].latin.clone()),
                        media_type,
                    },
                )),
                None => Some(ScanItem::unmatched(CacheEntry {
                    path: path_str,
                    mtime,
                    species_latin: None,
                    media_type,
                })),
            }
        })
        .collect();

    let mut matches = Vec::new();
    let mut cache_entries = Vec::with_capacity(results.len());
    let mut total_images = 0usize;
    let mut total_videos = 0usize;
    let mut matched_images = 0usize;
    let mut matched_videos = 0usize;
    let mut matched_species = HashSet::new();
    let mut seen_paths = HashSet::new();

    for item in results {
        if !seen_paths.insert(item.cache_entry.path.clone()) {
            continue;
        }
        match item.cache_entry.media_type {
            MediaType::Image => total_images += 1,
            MediaType::Video => total_videos += 1,
        }
        cache_entries.push(item.cache_entry);
        if let Some(media) = item.matched_media {
            match media.media_type {
                MediaType::Image => matched_images += 1,
                MediaType::Video => matched_videos += 1,
            }
            matched_species.insert(media.species_idx);
            matches.push(media);
        }
    }

    let total_media = total_images + total_videos;
    let matched_media = matched_images + matched_videos;
    let unmatched_images = total_images.saturating_sub(matched_images);
    let unmatched_videos = total_videos.saturating_sub(matched_videos);
    let unmatched_media = unmatched_images + unmatched_videos;

    ScanOutput {
        matches,
        cache_entries,
        stats: ScanStats {
            total_media,
            total_images,
            total_videos,
            matched_media,
            matched_images,
            matched_videos,
            matched_species: matched_species.len(),
            unmatched_media,
            unmatched_images,
            unmatched_videos,
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

fn media_type(path: &Path) -> Option<MediaType> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "heic" => Some(MediaType::Image),
        "mp4" | "mov" | "m4v" | "avi" | "mkv" | "webm" | "mts" | "m2ts" => Some(MediaType::Video),
        _ => None,
    }
}

struct ScanItem {
    matched_media: Option<MatchedMedia>,
    cache_entry: CacheEntry,
}

impl ScanItem {
    fn matched(matched_media: MatchedMedia, cache_entry: CacheEntry) -> Self {
        Self {
            matched_media: Some(matched_media),
            cache_entry,
        }
    }

    fn unmatched(cache_entry: CacheEntry) -> Self {
        Self {
            matched_media: None,
            cache_entry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cache::CacheIndex;

    #[test]
    fn overlapping_roots_do_not_count_the_same_media_twice() {
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

        assert_eq!(output.stats.total_media, 1);
        assert_eq!(output.stats.total_images, 1);
        assert_eq!(output.stats.total_videos, 0);
        assert_eq!(output.stats.matched_media, 1);
        assert_eq!(output.stats.matched_species, 1);
        assert_eq!(output.matches.len(), 1);

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn scans_and_classifies_supported_images_and_videos() {
        let base = std::env::temp_dir().join(format!(
            "birdindex2-mixed-media-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("白头鹎.jpg"), b"image").unwrap();
        std::fs::write(base.join("白头鹎.MP4"), b"video").unwrap();
        std::fs::write(base.join("unknown.mov"), b"video").unwrap();
        std::fs::write(base.join("白头鹎.txt"), b"unsupported").unwrap();

        let entries = vec![IocEntry {
            order: "PASSERIFORMES".into(),
            family: "Pycnonotidae".into(),
            latin: "Pycnonotus sinensis".into(),
            chinese: "白头鹎".into(),
        }];
        let latin_index = HashMap::from([("pycnonotus sinensis".into(), 0)]);
        let matcher = NameMatcher::new(&entries);
        let roots = vec![base.to_string_lossy().to_string()];

        let output = scan_paths(
            &roots,
            &entries,
            &latin_index,
            &matcher,
            &CacheIndex::empty(),
        );

        assert_eq!(output.stats.total_media, 3);
        assert_eq!(output.stats.total_images, 1);
        assert_eq!(output.stats.total_videos, 2);
        assert_eq!(output.stats.matched_media, 2);
        assert_eq!(output.stats.matched_images, 1);
        assert_eq!(output.stats.matched_videos, 1);
        assert_eq!(output.stats.unmatched_media, 1);
        assert_eq!(output.stats.unmatched_images, 0);
        assert_eq!(output.stats.unmatched_videos, 1);
        assert_eq!(output.stats.matched_species, 1);
        assert_eq!(output.matches.len(), 2);
        assert!(output
            .matches
            .iter()
            .any(|media| media.media_type == MediaType::Video));

        let warm_cache = CacheIndex {
            entries: output
                .cache_entries
                .iter()
                .cloned()
                .map(|entry| (entry.path.clone(), entry))
                .collect(),
        };
        let cached_output = scan_paths(&roots, &entries, &latin_index, &matcher, &warm_cache);
        assert_eq!(cached_output.stats.total_media, output.stats.total_media);
        assert_eq!(cached_output.stats.total_images, output.stats.total_images);
        assert_eq!(cached_output.stats.total_videos, output.stats.total_videos);
        assert_eq!(
            cached_output.stats.matched_media,
            output.stats.matched_media
        );
        assert_eq!(
            cached_output.stats.matched_images,
            output.stats.matched_images
        );
        assert_eq!(
            cached_output.stats.matched_videos,
            output.stats.matched_videos
        );
        assert_eq!(
            cached_output.stats.unmatched_media,
            output.stats.unmatched_media
        );
        assert_eq!(
            cached_output.stats.matched_species,
            output.stats.matched_species
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn recognizes_all_supported_video_extensions_case_insensitively() {
        for extension in ["mp4", "MOV", "m4v", "AVI", "mkv", "WEBM", "mts", "M2TS"] {
            let path = PathBuf::from(format!("bird.{extension}"));
            assert_eq!(media_type(&path), Some(MediaType::Video));
        }
        assert_eq!(media_type(Path::new("bird.raw")), None);
    }
}
