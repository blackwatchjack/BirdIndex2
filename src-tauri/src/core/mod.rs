pub mod cache;
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
use tree::build_tree;
use types::{ScanRequest, ScanResponse};

pub fn scan_and_build(request: ScanRequest) -> Result<ScanResponse> {
    let ioc = IocDatabase::load(&request.ioc_path)?;
    let ioc_fingerprint = fingerprint(&request.ioc_path)?;
    let cache = load_cache(&request.cache_path, &ioc_fingerprint)?;
    let matcher = NameMatcher::new(&ioc.entries);

    let output = scan_paths(
        &request.roots,
        &ioc.entries,
        &ioc.latin_index,
        &matcher,
        &cache,
    );
    let tree = build_tree(&ioc.entries, &output.matches);

    save_cache(&request.cache_path, &ioc_fingerprint, output.cache_entries)?;

    Ok(ScanResponse {
        tree,
        stats: output.stats,
        total_species: ioc.entries.len(),
    })
}
