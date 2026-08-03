use crate::core::types::{
    FamilyNode, GenusNode, IocEntry, MatchedMedia, MediaItem, OrderNode, SpeciesNode, TaxonTree,
};
use std::collections::HashMap;

pub fn build_tree(entries: &[IocEntry], matches: &[MatchedMedia]) -> TaxonTree {
    let mut orders: HashMap<String, OrderAgg> = HashMap::new();

    for matched in matches {
        let entry = &entries[matched.species_idx];
        let genus = genus_name(&entry.latin);

        let order = orders.entry(entry.order.clone()).or_default();
        let family = order.families.entry(entry.family.clone()).or_default();
        let genus_node = family.genera.entry(genus).or_default();
        let species = genus_node
            .species
            .entry(entry.latin.clone())
            .or_insert_with(|| SpeciesAgg {
                latin: entry.latin.clone(),
                chinese: entry.chinese.clone(),
                media_items: Vec::new(),
            });

        species.media_items.push(MediaItem {
            path: matched.path.clone(),
            file_name: matched.file_name.clone(),
            media_type: matched.media_type,
        });
    }

    let mut order_nodes: Vec<OrderNode> = orders
        .into_iter()
        .map(|(name, agg)| agg.into_node(name))
        .collect();
    order_nodes.sort_by(|a, b| a.name.cmp(&b.name));

    TaxonTree {
        orders: order_nodes,
    }
}

fn genus_name(latin: &str) -> String {
    latin
        .split_whitespace()
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

#[derive(Default)]
struct OrderAgg {
    families: HashMap<String, FamilyAgg>,
}

impl OrderAgg {
    fn into_node(self, name: String) -> OrderNode {
        let mut families: Vec<FamilyNode> = self
            .families
            .into_iter()
            .map(|(name, agg)| agg.into_node(name))
            .collect();
        families.sort_by(|a, b| a.name.cmp(&b.name));
        let media_count = families.iter().map(|family| family.media_count).sum();
        OrderNode {
            name,
            media_count,
            families,
        }
    }
}

#[derive(Default)]
struct FamilyAgg {
    genera: HashMap<String, GenusAgg>,
}

impl FamilyAgg {
    fn into_node(self, name: String) -> FamilyNode {
        let mut genera: Vec<GenusNode> = self
            .genera
            .into_iter()
            .map(|(name, agg)| agg.into_node(name))
            .collect();
        genera.sort_by(|a, b| a.name.cmp(&b.name));
        let media_count = genera.iter().map(|genus| genus.media_count).sum();
        FamilyNode {
            name,
            media_count,
            genera,
        }
    }
}

#[derive(Default)]
struct GenusAgg {
    species: HashMap<String, SpeciesAgg>,
}

impl GenusAgg {
    fn into_node(self, name: String) -> GenusNode {
        let mut species: Vec<SpeciesNode> = self
            .species
            .into_values()
            .map(|agg| agg.into_node())
            .collect();
        species.sort_by(|a, b| a.latin.cmp(&b.latin));
        let media_count = species.iter().map(|species| species.media_count).sum();
        GenusNode {
            name,
            media_count,
            species,
        }
    }
}

struct SpeciesAgg {
    latin: String,
    chinese: String,
    media_items: Vec<MediaItem>,
}

impl SpeciesAgg {
    fn into_node(mut self) -> SpeciesNode {
        self.media_items.sort_by(|left, right| {
            left.file_name
                .cmp(&right.file_name)
                .then_with(|| left.path.cmp(&right.path))
        });
        let media_count = self.media_items.len();
        SpeciesNode {
            latin: self.latin,
            chinese: self.chinese,
            media_count,
            media_items: self.media_items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::MediaType;

    #[test]
    fn aggregates_images_and_videos_into_media_counts() {
        let entries = vec![IocEntry {
            order: "PASSERIFORMES".into(),
            family: "Pycnonotidae".into(),
            latin: "Pycnonotus sinensis".into(),
            chinese: "白头鹎".into(),
        }];
        let matches = vec![
            MatchedMedia {
                path: "/media/白头鹎.jpg".into(),
                file_name: "白头鹎.jpg".into(),
                media_type: MediaType::Image,
                species_idx: 0,
            },
            MatchedMedia {
                path: "/media/白头鹎.mp4".into(),
                file_name: "白头鹎.mp4".into(),
                media_type: MediaType::Video,
                species_idx: 0,
            },
        ];

        let tree = build_tree(&entries, &matches);
        let order = &tree.orders[0];
        let family = &order.families[0];
        let genus = &family.genera[0];
        let species = &genus.species[0];

        assert_eq!(order.media_count, 2);
        assert_eq!(family.media_count, 2);
        assert_eq!(genus.media_count, 2);
        assert_eq!(species.media_count, 2);
        assert_eq!(species.media_items.len(), 2);
        assert!(species
            .media_items
            .iter()
            .any(|media| media.media_type == MediaType::Video));
    }
}
