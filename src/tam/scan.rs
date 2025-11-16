// Scan implementation for Kasate table access method

use pgrx::prelude::*;
use crate::storage::{TupleId, Tuple, STORAGE};
use crate::bridge;
use std::sync::Arc;

/// Scan descriptor - stores state for sequential scans
pub struct KasateScanDesc {
    pub relation_oid: u32,
    pub snapshot_xmin: u32,
    pub snapshot_xmax: u32,
    pub iterator_state: Option<IteratorState>,
}

/// Iterator state for scanning
pub struct IteratorState {
    pub tuples: Vec<(TupleId, Tuple)>,
    pub current_index: usize,
}

impl KasateScanDesc {
    pub fn new(relation_oid: u32, snapshot_xmin: u32, snapshot_xmax: u32) -> Self {
        Self {
            relation_oid,
            snapshot_xmin,
            snapshot_xmax,
            iterator_state: None,
        }
    }

    pub fn init_scan(&mut self) {
        // Collect all visible tuples into a vector for iteration
        let storage = STORAGE.get_or_create_relation(self.relation_oid);
        let storage_guard = storage.read().unwrap();

        let mut tuples = Vec::new();
        storage_guard.scan_visible(
            self.snapshot_xmin,
            self.snapshot_xmax,
            |tid, tuple| {
                tuples.push((*tid, tuple.clone()));
                true
            },
        );

        self.iterator_state = Some(IteratorState {
            tuples,
            current_index: 0,
        });
    }

    pub fn next_tuple(&mut self) -> Option<&Tuple> {
        if let Some(ref mut state) = self.iterator_state {
            if state.current_index < state.tuples.len() {
                let tuple_ref = &state.tuples[state.current_index].1;
                state.current_index += 1;
                Some(tuple_ref)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn rescan(&mut self) {
        if let Some(ref mut state) = self.iterator_state {
            state.current_index = 0;
        } else {
            self.init_scan();
        }
    }
}

/// Index fetch descriptor - stores state for index scans
pub struct KasateIndexFetchDesc {
    pub relation_oid: u32,
    pub snapshot_xmin: u32,
    pub snapshot_xmax: u32,
}

impl KasateIndexFetchDesc {
    pub fn new(relation_oid: u32, snapshot_xmin: u32, snapshot_xmax: u32) -> Self {
        Self {
            relation_oid,
            snapshot_xmin,
            snapshot_xmax,
        }
    }

    pub fn fetch_tuple(&self, tid: TupleId) -> Option<Tuple> {
        let storage = STORAGE.get_or_create_relation(self.relation_oid);
        let storage_guard = storage.read().unwrap();

        if let Some(tuple) = storage_guard.get(tid) {
            if tuple.is_visible(self.snapshot_xmin, self.snapshot_xmax) {
                Some(tuple.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::RelationStorage;

    #[test]
    fn test_scan_desc_creation() {
        let scan = KasateScanDesc::new(123, 0, 100);
        assert_eq!(scan.relation_oid, 123);
        assert_eq!(scan.snapshot_xmin, 0);
        assert_eq!(scan.snapshot_xmax, 100);
    }

    #[test]
    fn test_scan_iteration() {
        // Create a test relation and add some tuples
        let storage = STORAGE.get_or_create_relation(999);
        {
            let mut storage_guard = storage.write().unwrap();
            storage_guard.insert(vec![1, 2, 3], 50);
            storage_guard.insert(vec![4, 5, 6], 60);
            storage_guard.insert(vec![7, 8, 9], 70);
        }

        let mut scan = KasateScanDesc::new(999, 0, 100);
        scan.init_scan();

        let mut count = 0;
        while let Some(_tuple) = scan.next_tuple() {
            count += 1;
        }

        assert_eq!(count, 3);
    }

    #[test]
    fn test_scan_rescan() {
        let storage = STORAGE.get_or_create_relation(998);
        {
            let mut storage_guard = storage.write().unwrap();
            storage_guard.insert(vec![1, 2, 3], 50);
        }

        let mut scan = KasateScanDesc::new(998, 0, 100);
        scan.init_scan();

        // First scan
        let tuple1 = scan.next_tuple();
        assert!(tuple1.is_some());

        let tuple2 = scan.next_tuple();
        assert!(tuple2.is_none());

        // Rescan
        scan.rescan();
        let tuple3 = scan.next_tuple();
        assert!(tuple3.is_some());
    }
}
