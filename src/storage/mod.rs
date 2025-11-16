// Storage module - implements safe Rust storage backend using BTreeMap
// This module bridges from C FFI to safe Rust as quickly as possible

pub mod tuple;
pub mod relation;
pub mod transaction;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Tuple identifier - uniquely identifies a tuple within a relation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleId {
    pub block: u32,
    pub offset: u16,
}

impl TupleId {
    pub fn new(block: u32, offset: u16) -> Self {
        Self { block, offset }
    }

    pub fn to_item_pointer(&self) -> (u32, u16) {
        (self.block, self.offset)
    }

    pub fn from_item_pointer(block: u32, offset: u16) -> Self {
        Self::new(block, offset)
    }
}

/// Safe Rust representation of a tuple
#[derive(Debug, Clone)]
pub struct Tuple {
    pub id: TupleId,
    pub data: Vec<u8>,
    pub xmin: u32,  // Transaction ID that inserted this tuple
    pub xmax: u32,  // Transaction ID that deleted this tuple (0 if not deleted)
    pub cid: u32,   // Command ID
}

impl Tuple {
    pub fn new(id: TupleId, data: Vec<u8>, xmin: u32) -> Self {
        Self {
            id,
            data,
            xmin,
            xmax: 0,
            cid: 0,
        }
    }

    pub fn is_visible(&self, snapshot_xmin: u32, snapshot_xmax: u32) -> bool {
        // Simplified visibility check
        // Tuple is visible if it was inserted before the snapshot
        // and not deleted or deleted after the snapshot
        self.xmin < snapshot_xmax && (self.xmax == 0 || self.xmax >= snapshot_xmin)
    }
}

/// Relation storage - maps tuple IDs to tuples
pub struct RelationStorage {
    tuples: BTreeMap<TupleId, Tuple>,
    next_block: u32,
    next_offset: u16,
}

impl RelationStorage {
    pub fn new() -> Self {
        Self {
            tuples: BTreeMap::new(),
            next_block: 0,
            next_offset: 0,
        }
    }

    pub fn insert(&mut self, data: Vec<u8>, xmin: u32) -> TupleId {
        let id = self.allocate_tuple_id();
        let tuple = Tuple::new(id, data, xmin);
        self.tuples.insert(id, tuple);
        pgrx::warning!("[KASATE-STORAGE] Inserted tuple: tid=({},{}) xmin={}, total_tuples={}",
            id.block, id.offset, xmin, self.tuples.len());
        id
    }

    pub fn update(&mut self, id: TupleId, data: Vec<u8>, xmin: u32, xmax: u32) -> Option<TupleId> {
        // Mark old tuple as deleted
        if let Some(old_tuple) = self.tuples.get_mut(&id) {
            old_tuple.xmax = xmax;
            // Insert new version
            let new_id = self.insert(data, xmin);
            Some(new_id)
        } else {
            None
        }
    }

    pub fn delete(&mut self, id: TupleId, xmax: u32) -> bool {
        if let Some(tuple) = self.tuples.get_mut(&id) {
            tuple.xmax = xmax;
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: TupleId) -> Option<&Tuple> {
        self.tuples.get(&id)
    }

    pub fn scan<F>(&self, mut f: F)
    where
        F: FnMut(&TupleId, &Tuple) -> bool
    {
        for (id, tuple) in &self.tuples {
            if !f(id, tuple) {
                break;
            }
        }
    }

    pub fn scan_visible<F>(&self, snapshot_xmin: u32, snapshot_xmax: u32, mut f: F)
    where
        F: FnMut(&TupleId, &Tuple) -> bool
    {
        for (id, tuple) in &self.tuples {
            if tuple.is_visible(snapshot_xmin, snapshot_xmax) {
                if !f(id, tuple) {
                    break;
                }
            }
        }
    }

    fn allocate_tuple_id(&mut self) -> TupleId {
        let id = TupleId::new(self.next_block, self.next_offset);

        // Advance to next position
        self.next_offset += 1;
        if self.next_offset >= 1000 {
            self.next_offset = 0;
            self.next_block += 1;
        }

        id
    }

    pub fn len(&self) -> usize {
        self.tuples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tuples.is_empty()
    }
}

impl Default for RelationStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Global storage manager - maps relation OIDs to relation storage
pub struct StorageManager {
    relations: RwLock<BTreeMap<u32, Arc<RwLock<RelationStorage>>>>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            relations: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn get_or_create_relation(&self, oid: u32) -> Arc<RwLock<RelationStorage>> {
        let relations = self.relations.read().unwrap();
        if let Some(storage) = relations.get(&oid) {
            return Arc::clone(storage);
        }
        drop(relations);

        // Need to create new relation storage
        let mut relations = self.relations.write().unwrap();
        // Double-check after acquiring write lock
        if let Some(storage) = relations.get(&oid) {
            return Arc::clone(storage);
        }

        let storage = Arc::new(RwLock::new(RelationStorage::new()));
        relations.insert(oid, Arc::clone(&storage));
        storage
    }

    pub fn drop_relation(&self, oid: u32) {
        let mut relations = self.relations.write().unwrap();
        relations.remove(&oid);
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

// Global storage manager instance
lazy_static::lazy_static! {
    pub static ref STORAGE: StorageManager = StorageManager::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_id_ordering() {
        let id1 = TupleId::new(0, 0);
        let id2 = TupleId::new(0, 1);
        let id3 = TupleId::new(1, 0);

        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    #[test]
    fn test_tuple_visibility() {
        let id = TupleId::new(0, 0);
        let tuple = Tuple::new(id, vec![1, 2, 3], 100);

        // Visible in snapshot that started after insertion
        assert!(tuple.is_visible(100, 200));

        // Not visible in snapshot that started before insertion
        assert!(!tuple.is_visible(50, 99));
    }

    #[test]
    fn test_relation_storage_insert() {
        let mut storage = RelationStorage::new();
        let id = storage.insert(vec![1, 2, 3], 100);

        let tuple = storage.get(id).unwrap();
        assert_eq!(tuple.data, vec![1, 2, 3]);
        assert_eq!(tuple.xmin, 100);
    }

    #[test]
    fn test_relation_storage_update() {
        let mut storage = RelationStorage::new();
        let id1 = storage.insert(vec![1, 2, 3], 100);
        let id2 = storage.update(id1, vec![4, 5, 6], 101, 100).unwrap();

        // Old tuple should be marked deleted
        let old_tuple = storage.get(id1).unwrap();
        assert_eq!(old_tuple.xmax, 100);

        // New tuple should exist
        let new_tuple = storage.get(id2).unwrap();
        assert_eq!(new_tuple.data, vec![4, 5, 6]);
        assert_eq!(new_tuple.xmin, 101);
    }

    #[test]
    fn test_relation_storage_delete() {
        let mut storage = RelationStorage::new();
        let id = storage.insert(vec![1, 2, 3], 100);

        assert!(storage.delete(id, 101));

        let tuple = storage.get(id).unwrap();
        assert_eq!(tuple.xmax, 101);
    }

    #[test]
    fn test_relation_storage_scan() {
        let mut storage = RelationStorage::new();
        storage.insert(vec![1, 2, 3], 100);
        storage.insert(vec![4, 5, 6], 101);
        storage.insert(vec![7, 8, 9], 102);

        let mut count = 0;
        storage.scan(|_, _| {
            count += 1;
            true
        });

        assert_eq!(count, 3);
    }

    #[test]
    fn test_storage_manager() {
        let manager = StorageManager::new();

        let storage1 = manager.get_or_create_relation(1);
        let storage2 = manager.get_or_create_relation(1);

        // Should return the same storage
        assert!(Arc::ptr_eq(&storage1, &storage2));

        // Different OID should return different storage
        let storage3 = manager.get_or_create_relation(2);
        assert!(!Arc::ptr_eq(&storage1, &storage3));
    }
}
