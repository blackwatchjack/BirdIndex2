use crate::core::types::IocEntry;
use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use std::collections::HashMap;
use std::path::Path;

pub struct IocDatabase {
    pub entries: Vec<IocEntry>,
    pub latin_index: HashMap<String, usize>,
}

impl IocDatabase {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut workbook = open_workbook_auto(path)
            .with_context(|| format!("Failed to open IOC workbook: {}", path.display()))?;

        let range = workbook
            .worksheet_range("List")
            .with_context(|| "Worksheet 'List' not found")?;

        let mut rows = range.rows();
        let header = rows
            .next()
            .ok_or_else(|| anyhow!("Worksheet 'List' is empty"))?;

        let mut col_map = HashMap::new();
        for (idx, cell) in header.iter().enumerate() {
            if let Some(name) = cell_string(cell) {
                col_map.insert(name, idx);
            }
        }

        let order_col = *col_map
            .get("Order")
            .ok_or_else(|| anyhow!("Column 'Order' not found"))?;
        let family_col = *col_map
            .get("Family")
            .ok_or_else(|| anyhow!("Column 'Family' not found"))?;
        let latin_col = *col_map
            .get("IOC_15.1")
            .ok_or_else(|| anyhow!("Column 'IOC_15.1' not found"))?;
        let chinese_col = *col_map
            .get("Chinese")
            .ok_or_else(|| anyhow!("Column 'Chinese' not found"))?;

        let mut entries = Vec::new();
        for row in rows {
            let order = cell_string(row.get(order_col).unwrap_or(&DataType::Empty));
            let family = cell_string(row.get(family_col).unwrap_or(&DataType::Empty));
            let latin = cell_string(row.get(latin_col).unwrap_or(&DataType::Empty));
            let chinese = cell_string(row.get(chinese_col).unwrap_or(&DataType::Empty))
                .unwrap_or_default();

            let (order, family, latin) = match (order, family, latin) {
                (Some(order), Some(family), Some(latin)) => (order, family, latin),
                _ => continue,
            };

            let latin = latin.trim().to_string();
            if latin.is_empty() {
                continue;
            }

            entries.push(IocEntry {
                order: order.trim().to_string(),
                family: family.trim().to_string(),
                latin,
                chinese: chinese.trim().to_string(),
            });
        }

        let mut latin_index = HashMap::with_capacity(entries.len());
        for (idx, entry) in entries.iter().enumerate() {
            latin_index.insert(entry.latin.to_lowercase(), idx);
        }

        Ok(Self {
            entries,
            latin_index,
        })
    }
}

fn cell_string(cell: &DataType) -> Option<String> {
    match cell {
        DataType::String(value) => Some(value.trim().to_string()),
        DataType::Float(value) => Some(format!("{}", value)),
        DataType::Int(value) => Some(value.to_string()),
        DataType::Bool(value) => Some(value.to_string()),
        DataType::Error(_) | DataType::Empty => None,
        DataType::DateTime(value) => Some(value.to_string()),
        DataType::Duration(value) => Some(value.to_string()),
        DataType::DateTimeIso(value) => Some(value.to_string()),
        DataType::DurationIso(value) => Some(value.to_string()),
    }
}
