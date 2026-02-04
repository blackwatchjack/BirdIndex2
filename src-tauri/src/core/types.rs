use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IocEntry {
    pub order: String,
    pub family: String,
    pub latin: String,
    pub chinese: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoItem {
    pub path: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesNode {
    pub latin: String,
    pub chinese: String,
    pub count: usize,
    pub photos: Vec<PhotoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenusNode {
    pub name: String,
    pub count: usize,
    pub species: Vec<SpeciesNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyNode {
    pub name: String,
    pub count: usize,
    pub genera: Vec<GenusNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderNode {
    pub name: String,
    pub count: usize,
    pub families: Vec<FamilyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonTree {
    pub orders: Vec<OrderNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_files: usize,
    pub matched_files: usize,
    pub unmatched_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub tree: TaxonTree,
    pub stats: ScanStats,
    pub total_species: usize,
}

#[derive(Debug, Clone)]
pub struct MatchedPhoto {
    pub path: String,
    pub file_name: String,
    pub species_idx: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub path: String,
    pub mtime: i64,
    pub species_latin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFile {
    pub version: u32,
    pub ioc_fingerprint: String,
    pub entries: Vec<CacheEntry>,
}

impl CacheFile {
}
