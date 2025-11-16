// Kasate - A PostgreSQL table access method implementation using PGRX
// This extension replaces the heap storage with a BTreeMap-based storage in safe Rust

use pgrx::prelude::*;

// Modules
mod storage;
mod bridge;
mod tam;

// Re-export for easier access
pub use storage::STORAGE;

// Initialize PGRX
pgrx::pg_module_magic!();

/// Extension initialization
#[pg_guard]
pub extern "C" fn _PG_init() {
    // Initialize the table access method
    tam::init_tableam();
}

/// Table access method handler - entry point for Postgres
#[pg_extern(sql = r#"
    CREATE FUNCTION kasate_handler(internal) RETURNS table_am_handler
    LANGUAGE c AS 'MODULE_PATHNAME', 'kasate_tableam_handler';
"#)]
fn kasate_tableam_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    tam::kasate_tableam_handler(fcinfo)
}

/// Create the table access method in SQL
#[pg_extern(sql = r#"
    CREATE ACCESS METHOD kasate TYPE TABLE HANDLER kasate_handler;
"#)]
fn create_kasate_am() {
    // This is just a marker function, the actual creation happens via SQL
}

/// Helper function to get storage statistics
#[pg_extern]
fn kasate_storage_stats(relation_oid: pg_sys::Oid) -> TableIterator<'static, (name!(oid, pg_sys::Oid), name!(tuple_count, i64))> {
    let storage = STORAGE.get_or_create_relation(relation_oid);
    let storage_guard = storage.read().unwrap();
    let count = storage_guard.len() as i64;

    TableIterator::new(vec![(relation_oid, count)].into_iter())
}

/// Helper function to list all relations using Kasate
#[pg_extern]
fn kasate_list_relations() -> TableIterator<'static, (name!(relname, String), name!(oid, pg_sys::Oid), name!(tuple_count, i64))> {
    // This is a simplified version - in production, you'd query pg_class
    // For now, return empty result
    TableIterator::new(vec![].into_iter())
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;
    use crate::storage::{TupleId, RelationStorage, STORAGE};

    #[pg_test]
    fn test_storage_insert() {
        let storage = STORAGE.get_or_create_relation(12345);
        let mut storage_guard = storage.write().unwrap();

        let tid = storage_guard.insert(vec![1, 2, 3, 4], 100);
        assert!(storage_guard.get(tid).is_some());
    }

    #[pg_test]
    fn test_storage_update() {
        let storage = STORAGE.get_or_create_relation(12346);
        let mut storage_guard = storage.write().unwrap();

        let tid1 = storage_guard.insert(vec![1, 2, 3], 100);
        let tid2 = storage_guard.update(tid1, vec![4, 5, 6], 101, 100);

        assert!(tid2.is_some());
        let old_tuple = storage_guard.get(tid1).unwrap();
        assert_eq!(old_tuple.xmax, 100);
    }

    #[pg_test]
    fn test_storage_delete() {
        let storage = STORAGE.get_or_create_relation(12347);
        let mut storage_guard = storage.write().unwrap();

        let tid = storage_guard.insert(vec![1, 2, 3], 100);
        assert!(storage_guard.delete(tid, 101));

        let tuple = storage_guard.get(tid).unwrap();
        assert_eq!(tuple.xmax, 101);
    }

    #[pg_test]
    fn test_storage_scan() {
        let storage = STORAGE.get_or_create_relation(12348);
        {
            let mut storage_guard = storage.write().unwrap();
            storage_guard.insert(vec![1], 100);
            storage_guard.insert(vec![2], 101);
            storage_guard.insert(vec![3], 102);
        }

        let storage_guard = storage.read().unwrap();
        let mut count = 0;
        storage_guard.scan(|_, _| {
            count += 1;
            true
        });

        assert_eq!(count, 3);
    }

    #[pg_test]
    fn test_tuple_visibility() {
        let storage = STORAGE.get_or_create_relation(12349);
        {
            let mut storage_guard = storage.write().unwrap();
            storage_guard.insert(vec![1], 100);  // Inserted at xid 100
            storage_guard.insert(vec![2], 150);  // Inserted at xid 150
        }

        let storage_guard = storage.read().unwrap();
        let mut visible_count = 0;

        // Snapshot from 120 to 200 should see only first tuple
        storage_guard.scan_visible(120, 200, |_, tuple| {
            visible_count += 1;
            true
        });

        assert_eq!(visible_count, 1);
    }

    #[pg_test]
    fn test_multiple_relations() {
        let storage1 = STORAGE.get_or_create_relation(1001);
        let storage2 = STORAGE.get_or_create_relation(1002);

        {
            let mut guard1 = storage1.write().unwrap();
            let mut guard2 = storage2.write().unwrap();

            guard1.insert(vec![1], 100);
            guard2.insert(vec![2], 100);
        }

        let guard1 = storage1.read().unwrap();
        let guard2 = storage2.read().unwrap();

        assert_eq!(guard1.len(), 1);
        assert_eq!(guard2.len(), 1);
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // Initialize test environment
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
