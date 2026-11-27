use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IocEntry {
    pub order: String,
    pub family: String,
    pub latin: String,
    pub chinese: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    #[default]
    Image,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub path: String,
    pub file_name: String,
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesNode {
    pub latin: String,
    pub chinese: String,
    pub media_count: usize,
    pub media_items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenusNode {
    pub name: String,
    pub media_count: usize,
    pub species: Vec<SpeciesNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyNode {
    pub name: String,
    pub media_count: usize,
    pub genera: Vec<GenusNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderNode {
    pub name: String,
    pub media_count: usize,
    pub families: Vec<FamilyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonTree {
    pub orders: Vec<OrderNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_media: usize,
    pub total_images: usize,
    pub total_videos: usize,
    pub matched_media: usize,
    pub matched_images: usize,
    pub matched_videos: usize,
    pub matched_species: usize,
    pub unmatched_media: usize,
    pub unmatched_images: usize,
    pub unmatched_videos: usize,
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
    pub roots: Vec<String>,
    pub ioc_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub destination: String,
    pub exported_at: String,
    pub scan: ScanResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub path: String,
    pub species_count: usize,
}

#[derive(Debug, Clone)]
pub struct MatchedMedia {
    pub path: String,
    pub file_name: String,
    pub media_type: MediaType,
    pub species_idx: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub path: String,
    pub mtime: i64,
    pub species_latin: Option<String>,
    #[serde(default)]
    pub media_type: MediaType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFile {
    pub version: u32,
    pub ioc_fingerprint: String,
    pub entries: Vec<CacheEntry>,
}
