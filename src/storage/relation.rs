// Safe Rust types for relation metadata

/// Relation metadata - safe representation
#[derive(Debug, Clone)]
pub struct RelationMetadata {
    pub oid: u32,
    pub name: String,
    pub relkind: char,
    pub relpages: u32,
    pub reltuples: f32,
}

impl RelationMetadata {
    pub fn new(oid: u32, name: String) -> Self {
        Self {
            oid,
            name,
            relkind: 'r', // ordinary table
            relpages: 0,
            reltuples: 0.0,
        }
    }

    pub fn update_stats(&mut self, pages: u32, tuples: f32) {
        self.relpages = pages;
        self.reltuples = tuples;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_metadata() {
        let mut meta = RelationMetadata::new(123, "test_table".to_string());
        assert_eq!(meta.oid, 123);
        assert_eq!(meta.name, "test_table");
        assert_eq!(meta.relpages, 0);

        meta.update_stats(10, 1000.0);
        assert_eq!(meta.relpages, 10);
        assert_eq!(meta.reltuples, 1000.0);
    }
}
