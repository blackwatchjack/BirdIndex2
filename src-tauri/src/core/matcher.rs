use crate::core::types::IocEntry;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

pub struct NameMatcher {
    latin: Option<AhoCorasick>,
    latin_map: Vec<usize>,
    chinese: Option<AhoCorasick>,
    chinese_map: Vec<usize>,
}

impl NameMatcher {
    pub fn new(entries: &[IocEntry]) -> Self {
        let mut latin_patterns = Vec::new();
        let mut latin_map = Vec::new();
        let mut chinese_patterns = Vec::new();
        let mut chinese_map = Vec::new();

        for (idx, entry) in entries.iter().enumerate() {
            let latin = entry.latin.trim();
            if !latin.is_empty() {
                latin_patterns.push(latin.to_lowercase());
                latin_map.push(idx);
            }
            let chinese = entry.chinese.trim();
            if !chinese.is_empty() {
                chinese_patterns.push(chinese.to_lowercase());
                chinese_map.push(idx);
            }
        }

        let latin = if latin_patterns.is_empty() {
            None
        } else {
            AhoCorasickBuilder::new()
                .match_kind(MatchKind::LeftmostFirst)
                .build(latin_patterns)
                .ok()
        };

        let chinese = if chinese_patterns.is_empty() {
            None
        } else {
            AhoCorasickBuilder::new()
                .match_kind(MatchKind::LeftmostFirst)
                .build(chinese_patterns)
                .ok()
        };

        Self {
            latin,
            latin_map,
            chinese,
            chinese_map,
        }
    }

    pub fn match_name(&self, file_name: &str) -> Option<usize> {
        let name = file_name.to_lowercase();

        if let Some(latin) = &self.latin {
            if let Some(hit) = latin.find(&name) {
                return self.latin_map.get(hit.pattern().as_usize()).copied();
            }
        }

        if let Some(chinese) = &self.chinese {
            if let Some(hit) = chinese.find(&name) {
                return self.chinese_map.get(hit.pattern().as_usize()).copied();
            }
        }

        None
    }
}
