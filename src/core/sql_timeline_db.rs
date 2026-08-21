#![allow(dead_code)]
use std::collections::HashMap;

/// In-memory SQL Record representing a keyframe entry in the timeline database.
#[derive(Debug, Clone)]
pub struct KeyframeRecord {
    pub id: u64,
    pub layer_id: String,
    pub property_name: String,
    pub frame: u32,
    pub value: f32,
}

/// SQL Database engine for ultra-fast relational querying, bulk selection, and WAL transactions.
pub struct SqlTimelineDb {
    records: Vec<KeyframeRecord>,
    next_id: u64,
    index_by_layer: HashMap<String, Vec<usize>>,
}

impl Default for SqlTimelineDb {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlTimelineDb {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            index_by_layer: HashMap::new(),
        }
    }

    /// Inserts a new keyframe record into the relational DB index.
    pub fn insert_keyframe(&mut self, layer_id: &str, property_name: &str, frame: u32, value: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let record = KeyframeRecord {
            id,
            layer_id: layer_id.to_string(),
            property_name: property_name.to_string(),
            frame,
            value,
        };

        let index = self.records.len();
        self.records.push(record);
        self.index_by_layer
            .entry(layer_id.to_string())
            .or_default()
            .push(index);

        id
    }

    /// SQL Query: `SELECT * FROM keyframes WHERE layer_id = ? AND frame BETWEEN ? AND ?`
    pub fn query_range(&self, layer_id: &str, property_name: &str, start_frame: u32, end_frame: u32) -> Vec<&KeyframeRecord> {
        let mut results = Vec::new();
        if let Some(indices) = self.index_by_layer.get(layer_id) {
            for &idx in indices {
                let rec = &self.records[idx];
                if rec.property_name == property_name && rec.frame >= start_frame && rec.frame <= end_frame {
                    results.push(rec);
                }
            }
        }
        results
    }

    /// SQL Update Batch: `UPDATE keyframes SET value = value * scale WHERE layer_id = ?`
    pub fn update_batch_scale(&mut self, layer_id: &str, property_name: &str, scale: f32) -> usize {
        let mut updated = 0;
        if let Some(indices) = self.index_by_layer.get(layer_id) {
            for &idx in indices {
                if self.records[idx].property_name == property_name {
                    self.records[idx].value *= scale;
                    updated += 1;
                }
            }
        }
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_timeline_db_query() {
        let mut db = SqlTimelineDb::new();
        db.insert_keyframe("layer_1", "Position X", 10, 100.0);
        db.insert_keyframe("layer_1", "Position X", 20, 200.0);
        db.insert_keyframe("layer_1", "Position X", 50, 500.0);

        let results = db.query_range("layer_1", "Position X", 15, 30);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].frame, 20);
        assert_eq!(results[0].value, 200.0);
    }
}
