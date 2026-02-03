use crate::core::types::{
    FamilyNode, GenusNode, IocEntry, MatchedPhoto, OrderNode, PhotoItem, SpeciesNode,
    TaxonTree,
};
use std::collections::HashMap;

pub fn build_tree(entries: &[IocEntry], matches: &[MatchedPhoto]) -> TaxonTree {
    let mut orders: HashMap<String, OrderAgg> = HashMap::new();

    for matched in matches {
        let entry = &entries[matched.species_idx];
        let genus = genus_name(&entry.latin);

        let order = orders
            .entry(entry.order.clone())
            .or_insert_with(OrderAgg::default);
        let family = order
            .families
            .entry(entry.family.clone())
            .or_insert_with(FamilyAgg::default);
        let genus_node = family
            .genera
            .entry(genus)
            .or_insert_with(GenusAgg::default);
        let species = genus_node
            .species
            .entry(entry.latin.clone())
            .or_insert_with(|| SpeciesAgg {
                latin: entry.latin.clone(),
                chinese: entry.chinese.clone(),
                photos: Vec::new(),
            });

        species.photos.push(PhotoItem {
            path: matched.path.clone(),
            file_name: matched.file_name.clone(),
        });
    }

    let mut order_nodes: Vec<OrderNode> = orders
        .into_iter()
        .map(|(name, agg)| agg.into_node(name))
        .collect();
    order_nodes.sort_by(|a, b| a.name.cmp(&b.name));

    TaxonTree { orders: order_nodes }
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
        let count = families.iter().map(|f| f.count).sum();
        OrderNode {
            name,
            count,
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
        let count = genera.iter().map(|g| g.count).sum();
        FamilyNode { name, count, genera }
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
            .into_iter()
            .map(|(_, agg)| agg.into_node())
            .collect();
        species.sort_by(|a, b| a.latin.cmp(&b.latin));
        let count = species.iter().map(|s| s.count).sum();
        GenusNode {
            name,
            count,
            species,
        }
    }
}

struct SpeciesAgg {
    latin: String,
    chinese: String,
    photos: Vec<PhotoItem>,
}

impl SpeciesAgg {
    fn into_node(mut self) -> SpeciesNode {
        self.photos.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        let count = self.photos.len();
        SpeciesNode {
            latin: self.latin,
            chinese: self.chinese,
            count,
            photos: self.photos,
        }
    }
}
