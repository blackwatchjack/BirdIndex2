pub mod cache;
pub mod exporter;
pub mod ioc;
pub mod locator;
pub mod matcher;
pub mod scanner;
pub mod tree;
pub mod types;

use anyhow::Result;
use cache::{fingerprint, load_cache, save_cache};
use ioc::IocDatabase;
use matcher::NameMatcher;
use scanner::scan_paths;
use std::path::Path;
use tree::build_tree;
use types::{ScanRequest, ScanResponse};

pub fn scan_and_build(
    request: ScanRequest,
    ioc_path: &Path,
    cache_path: &Path,
) -> Result<ScanResponse> {
    let roots = request.roots;
    let ioc = IocDatabase::load(ioc_path)?;
    let ioc_fingerprint = fingerprint(ioc_path)?;
    let cache = load_cache(cache_path, &ioc_fingerprint)?;
    let matcher = NameMatcher::new(&ioc.entries);

    let output = scan_paths(&roots, &ioc.entries, &ioc.latin_index, &matcher, &cache);
    let tree = build_tree(&ioc.entries, &output.matches);

    save_cache(cache_path, &ioc_fingerprint, output.cache_entries)?;

    Ok(ScanResponse {
        tree,
        stats: output.stats,
        total_species: ioc.entries.len(),
        roots,
        ioc_source: ioc_path.to_string_lossy().to_string(),
    })
}
