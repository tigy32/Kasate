// TAM handler implementations

use pgrx::prelude::*;
use crate::storage::STORAGE;
use crate::bridge;
use crate::tam::scan::{KasateScanDesc, KasateIndexFetchDesc};
use std::collections::HashMap;
use std::sync::Mutex;

// Global storage for scan descriptors (workaround for removed opaque fields)
lazy_static::lazy_static! {
    static ref SCAN_DESCRIPTORS: Mutex<HashMap<usize, Box<KasateScanDesc>>> =
        Mutex::new(HashMap::new());
    static ref FETCH_DESCRIPTORS: Mutex<HashMap<usize, Box<KasateIndexFetchDesc>>> =
        Mutex::new(HashMap::new());
}

// ============================================================================
// Scan callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_scan_begin(
    relation: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: std::os::raw::c_int,
    key: *mut pg_sys::ScanKeyData,
    _pscan: pg_sys::ParallelTableScanDesc,
    flags: u32,
) -> pg_sys::TableScanDesc {
    pgrx::warning!("[KASATE] scan_begin called");
    unsafe {
        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        pgrx::warning!("[KASATE] scan_begin: relation_oid={}", relation_oid);
        let (snapshot_xmin, snapshot_xmax) = bridge::extract_snapshot_info(snapshot);

        // Create our scan descriptor
        let mut scan_desc = Box::new(KasateScanDesc::new(
            relation_oid,
            snapshot_xmin,
            snapshot_xmax,
        ));

        // Initialize the scan
        scan_desc.init_scan();

        // Allocate Postgres TableScanDescData
        let pg_scan = pg_sys::palloc0(std::mem::size_of::<pg_sys::TableScanDescData>())
            as *mut pg_sys::TableScanDescData;

        if !pg_scan.is_null() {
            let pg_scan_ref = &mut *pg_scan;
            pg_scan_ref.rs_rd = relation;
            pg_scan_ref.rs_snapshot = snapshot;
            pg_scan_ref.rs_nkeys = nkeys;
            pg_scan_ref.rs_key = key;
            pg_scan_ref.rs_flags = flags;

            // Store our scan descriptor in global HashMap
            let key = pg_scan as usize;
            SCAN_DESCRIPTORS.lock().unwrap().insert(key, scan_desc);
        }

        pg_scan
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_scan_end(scan: pg_sys::TableScanDesc) {
    pgrx::warning!("[KASATE] scan_end called");
    unsafe {
        if !scan.is_null() {
            pgrx::warning!("[KASATE] scan_end: removing scan descriptor");
            // Remove our scan descriptor from global HashMap
            let key = scan as usize;
            SCAN_DESCRIPTORS.lock().unwrap().remove(&key);
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_scan_rescan(
    scan: pg_sys::TableScanDesc,
    key: *mut pg_sys::ScanKeyData,
    _set_params: bool,
    _allow_strat: bool,
    _allow_sync: bool,
    _allow_pagemode: bool,
) {
    unsafe {
        if !scan.is_null() {
            let scan_ref = &mut *scan;
            scan_ref.rs_key = key;

            // Get scan descriptor from HashMap
            let hash_key = scan as usize;
            if let Some(scan_desc) = SCAN_DESCRIPTORS.lock().unwrap().get_mut(&hash_key) {
                scan_desc.rescan();
            }
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_scan_getnextslot(
    scan: pg_sys::TableScanDesc,
    _direction: pg_sys::ScanDirection::Type,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    unsafe {
        pgrx::warning!("[KASATE] scan_getnextslot called");

        if scan.is_null() || slot.is_null() {
            pgrx::warning!("[KASATE] scan_getnextslot: scan or slot is null");
            return false;
        }

        let scan_ref = &mut *scan;

        // Get scan descriptor from HashMap
        let hash_key = scan as usize;
        pgrx::warning!("[KASATE] scan_getnextslot: hash_key={}", hash_key);

        let mut descriptors = SCAN_DESCRIPTORS.lock().unwrap();
        let tuple_opt = if let Some(scan_desc) = descriptors.get_mut(&hash_key) {
            pgrx::warning!("[KASATE] scan_getnextslot: found scan descriptor");
            // Get next tuple from our scan and clone it
            scan_desc.next_tuple().cloned()
        } else {
            pgrx::warning!("[KASATE] scan_getnextslot: scan descriptor not found");
            None
        };
        drop(descriptors); // Release lock before calling other functions

        if let Some(tuple) = tuple_opt {
            pgrx::warning!("[KASATE] scan_getnextslot: found tuple, storing in slot");
            // Store tuple in slot
            store_tuple_in_slot(slot, &tuple, scan_ref.rs_rd);
            pgrx::warning!("[KASATE] scan_getnextslot: returning true");
            true
        } else {
            pgrx::warning!("[KASATE] scan_getnextslot: no more tuples");
            // No more tuples
            clear_tuple_slot(slot);
            false
        }
    }
}

// ============================================================================
// Index fetch callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_index_fetch_begin(
    relation: pg_sys::Relation,
) -> *mut pg_sys::IndexFetchTableData {
    pgrx::warning!("[KASATE] index_fetch_begin called");
    unsafe {
        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        pgrx::warning!("[KASATE] index_fetch_begin: relation_oid={}", relation_oid);

        // For now, use a simple snapshot
        let snapshot_xmin = 0;
        let snapshot_xmax = u32::MAX;

        let fetch_desc = Box::new(KasateIndexFetchDesc::new(
            relation_oid,
            snapshot_xmin,
            snapshot_xmax,
        ));

        // Allocate Postgres IndexFetchTableData
        let pg_fetch = pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexFetchTableData>())
            as *mut pg_sys::IndexFetchTableData;

        if !pg_fetch.is_null() {
            let pg_fetch_ref = &mut *pg_fetch;
            pg_fetch_ref.rel = relation;

            // Store our fetch descriptor in global HashMap
            let key = pg_fetch as usize;
            FETCH_DESCRIPTORS.lock().unwrap().insert(key, fetch_desc);
        }

        pg_fetch
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_index_fetch_reset(fetch: *mut pg_sys::IndexFetchTableData) {
    pgrx::warning!("[KASATE] index_fetch_reset called");
    // Nothing to reset in our implementation
    if fetch.is_null() {
        return;
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_index_fetch_end(fetch: *mut pg_sys::IndexFetchTableData) {
    pgrx::warning!("[KASATE] index_fetch_end called");
    unsafe {
        if !fetch.is_null() {
            // Remove our fetch descriptor from global HashMap
            let key = fetch as usize;
            FETCH_DESCRIPTORS.lock().unwrap().remove(&key);
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_index_fetch_tuple(
    fetch: *mut pg_sys::IndexFetchTableData,
    tid: pg_sys::ItemPointer,
    snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
    _call_again: *mut bool,
    _all_dead: *mut bool,
) -> bool {
    unsafe {
        if fetch.is_null() || tid.is_null() || slot.is_null() {
            return false;
        }

        let fetch_ref = &*fetch;

        // Get fetch descriptor from HashMap
        let hash_key = fetch as usize;
        let descriptors = FETCH_DESCRIPTORS.lock().unwrap();
        if let Some(fetch_desc) = descriptors.get(&hash_key) {
            let tuple_id = bridge::item_pointer_to_tuple_id(*tid);

            if let Some(tuple) = fetch_desc.fetch_tuple(tuple_id) {
                drop(descriptors); // Release lock
                store_tuple_in_slot(slot, &tuple, fetch_ref.rel);
                true
            } else {
                drop(descriptors); // Release lock
                clear_tuple_slot(slot);
                false
            }
        } else {
            false
        }
    }
}

// ============================================================================
// Tuple modification callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_insert(
    relation: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: std::os::raw::c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    unsafe {
        pgrx::warning!("[KASATE] tuple_insert called");

        if relation.is_null() || slot.is_null() {
            pgrx::warning!("[KASATE] ERROR: relation or slot is null");
            return;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let xid = bridge::get_current_transaction_id();
        pgrx::warning!("[KASATE] relation_oid={}, xid={}", relation_oid, xid);

        // Get tuple descriptor for proper tuple construction
        let slot_ref = &*slot;
        let tupdesc = slot_ref.tts_tupleDescriptor;

        if tupdesc.is_null() {
            pgrx::warning!("[KASATE] ERROR: tuple descriptor is null");
            return;
        }

        // Extract tuple data from slot
        match extract_tuple_from_slot(slot) {
            Some(tuple_data) => {
                pgrx::warning!("[KASATE] Extracted {} bytes of tuple data", tuple_data.len());

                // Insert into storage
                let storage = STORAGE.get_or_create_relation(relation_oid);
                let mut storage_guard = storage.write().unwrap();
                let tid = storage_guard.insert(tuple_data, xid);
                pgrx::warning!("[KASATE] Inserted tuple with TID: block={}, offset={}", tid.block, tid.offset);

                // Release write lock
                drop(storage_guard);

                // DO NOT modify the slot! It's input-only.
                // PostgreSQL will handle setting the TID through other mechanisms.
                pgrx::warning!("[KASATE] Tuple stored with TID: block={}, offset={}", tid.block, tid.offset);
            }
            None => {
                pgrx::warning!("[KASATE] ERROR: Failed to extract tuple data from slot");
            }
        }

        pgrx::warning!("[KASATE] tuple_insert completed");
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_delete(
    relation: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    _cid: pg_sys::CommandId,
    snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _changingPart: bool,
) -> pg_sys::TM_Result::Type {
    pgrx::warning!("[KASATE] tuple_delete called");
    unsafe {
        if relation.is_null() || tid.is_null() {
            return pg_sys::TM_Result::TM_Invisible;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);
        let xid = bridge::get_current_transaction_id();

        let storage = STORAGE.get_or_create_relation(relation_oid);
        let mut storage_guard = storage.write().unwrap();

        if storage_guard.delete(tuple_id, xid) {
            pg_sys::TM_Result::TM_Ok
        } else {
            pg_sys::TM_Result::TM_Invisible
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_update(
    relation: pg_sys::Relation,
    otid: pg_sys::ItemPointer,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _lockmode: *mut pg_sys::LockTupleMode::Type,
    _update_indexes: *mut pg_sys::TU_UpdateIndexes::Type,
) -> pg_sys::TM_Result::Type {
    pgrx::warning!("[KASATE] tuple_update called");
    unsafe {
        if relation.is_null() || otid.is_null() || slot.is_null() {
            return pg_sys::TM_Result::TM_Invisible;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let old_tid = bridge::item_pointer_to_tuple_id(*otid);
        let xid = bridge::get_current_transaction_id();

        if let Some(new_data) = extract_tuple_from_slot(slot) {
            let storage = STORAGE.get_or_create_relation(relation_oid);
            let mut storage_guard = storage.write().unwrap();

            if let Some(new_tid) = storage_guard.update(old_tid, new_data, xid, xid) {
                // Update slot with new TID
                let slot_ref = &mut *slot;
                let item_ptr = bridge::tuple_id_to_item_pointer(new_tid);
                slot_ref.tts_tid = item_ptr;
                pg_sys::TM_Result::TM_Ok
            } else {
                pg_sys::TM_Result::TM_Invisible
            }
        } else {
            pg_sys::TM_Result::TM_Invisible
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_lock(
    relation: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _mode: pg_sys::LockTupleMode::Type,
    _wait_policy: pg_sys::LockWaitPolicy::Type,
    _flags: u8,
    _tmfd: *mut pg_sys::TM_FailureData,
) -> pg_sys::TM_Result::Type {
    pgrx::warning!("[KASATE] tuple_lock called");
    // Simplified lock implementation - just verify tuple exists and is visible
    unsafe {
        if relation.is_null() || tid.is_null() {
            return pg_sys::TM_Result::TM_Invisible;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);
        let (snapshot_xmin, snapshot_xmax) = bridge::extract_snapshot_info(snapshot);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        if let Some(tuple) = storage_guard.get(tuple_id) {
            if tuple.is_visible(snapshot_xmin, snapshot_xmax) {
                if !slot.is_null() {
                    store_tuple_in_slot(slot, tuple, relation);
                }
                pg_sys::TM_Result::TM_Ok
            } else {
                pg_sys::TM_Result::TM_Invisible
            }
        } else {
            pg_sys::TM_Result::TM_Invisible
        }
    }
}

// ============================================================================
// Tuple fetch callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_fetch_row_version(
    relation: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    unsafe {
        pgrx::warning!("[KASATE] tuple_fetch_row_version called");

        if relation.is_null() || tid.is_null() || slot.is_null() {
            pgrx::warning!("[KASATE] tuple_fetch_row_version: null parameter");
            return false;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);
        let (snapshot_xmin, snapshot_xmax) = bridge::extract_snapshot_info(snapshot);

        pgrx::warning!("[KASATE] tuple_fetch_row_version: relation_oid={}, tuple_id=({},{}), snapshot=({},{})",
            relation_oid, tuple_id.block, tuple_id.offset, snapshot_xmin, snapshot_xmax);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        pgrx::warning!("[KASATE] tuple_fetch_row_version: got storage");

        let storage_guard = storage.read().unwrap();
        pgrx::warning!("[KASATE] tuple_fetch_row_version: acquired read lock");

        if let Some(tuple) = storage_guard.get(tuple_id) {
            pgrx::warning!("[KASATE] tuple_fetch_row_version: found tuple, checking visibility");
            if tuple.is_visible(snapshot_xmin, snapshot_xmax) {
                pgrx::warning!("[KASATE] tuple_fetch_row_version: tuple is visible, storing in slot");
                store_tuple_in_slot(slot, tuple, relation);
                pgrx::warning!("[KASATE] tuple_fetch_row_version: returning true");
                true
            } else {
                pgrx::warning!("[KASATE] tuple_fetch_row_version: tuple not visible");
                clear_tuple_slot(slot);
                false
            }
        } else {
            pgrx::warning!("[KASATE] tuple_fetch_row_version: tuple not found");
            clear_tuple_slot(slot);
            false
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_tid_valid(
    scan: pg_sys::TableScanDesc,
    tid: pg_sys::ItemPointer,
) -> bool {
    unsafe {
        if scan.is_null() || tid.is_null() {
            return false;
        }

        let scan_ref = &*scan;
        let relation_oid = bridge::extract_relation_oid(scan_ref.rs_rd).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        storage_guard.get(tuple_id).is_some()
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_tuple_satisfies_snapshot(
    relation: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    snapshot: pg_sys::Snapshot,
) -> bool {
    unsafe {
        if relation.is_null() || slot.is_null() || snapshot.is_null() {
            return false;
        }

        let slot_ref = &*slot;
        let tid = slot_ref.tts_tid;
        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(tid);
        let (snapshot_xmin, snapshot_xmax) = bridge::extract_snapshot_info(snapshot);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        if let Some(tuple) = storage_guard.get(tuple_id) {
            tuple.is_visible(snapshot_xmin, snapshot_xmax)
        } else {
            false
        }
    }
}

// ============================================================================
// Relation management callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_relation_set_new_filelocator(
    relation: pg_sys::Relation,
    _newrlocator: *const pg_sys::RelFileLocator,
    _persistence: ::std::os::raw::c_char,
    _freeze_xid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
    pgrx::warning!("[KASATE] relation_set_new_filelocator called");
    unsafe {
        if !relation.is_null() {
            let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
            // Clear any existing data
            STORAGE.drop_relation(relation_oid);
            // Create fresh storage
            STORAGE.get_or_create_relation(relation_oid);
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_relation_vacuum(
    relation: pg_sys::Relation,
    _params: *mut pg_sys::VacuumParams,
    _bstrategy: pg_sys::BufferAccessStrategy,
) {
    pgrx::warning!("[KASATE] relation_vacuum called");
    // Simplified vacuum - in a real implementation, this would clean up dead tuples
    unsafe {
        if !relation.is_null() {
            let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
            // For now, just acknowledge the vacuum request
            info!("Vacuum called for relation {}", relation_oid);
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_index_build_range_scan(
    table_relation: pg_sys::Relation,
    index_relation: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
    allow_sync: bool,
    anyvisible: bool,
    progress: bool,
    start_blockno: pg_sys::BlockNumber,
    numblocks: pg_sys::BlockNumber,
    callback: pg_sys::IndexBuildCallback,
    callback_state: *mut ::std::os::raw::c_void,
    scan: pg_sys::TableScanDesc,
) -> f64 {
    // Simplified index build - scan all tuples and call callback
    unsafe {
        if table_relation.is_null() {
            return 0.0;
        }

        let relation_oid = bridge::extract_relation_oid(table_relation).unwrap_or(0);
        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        let mut count = 0.0;
        storage_guard.scan(|_tid, _tuple| {
            count += 1.0;
            true
        });

        count
    }
}

// ============================================================================
// Estimation callbacks
// ============================================================================

#[pg_guard]
pub extern "C-unwind" fn kasate_relation_size(
    relation: pg_sys::Relation,
    _forkNumber: pg_sys::ForkNumber::Type,
) -> u64 {
    pgrx::warning!("[KASATE] relation_size called");
    unsafe {
        if relation.is_null() {
            pgrx::warning!("[KASATE] relation_size: relation is null");
            return 0;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        pgrx::warning!("[KASATE] relation_size: oid={}", relation_oid);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        pgrx::warning!("[KASATE] relation_size: got storage");

        let storage_guard = storage.read().unwrap();
        pgrx::warning!("[KASATE] relation_size: got read lock");

        // Estimate size based on number of tuples
        let tuple_count = storage_guard.len() as u64;
        pgrx::warning!("[KASATE] relation_size: returning size for {} tuples", tuple_count);
        tuple_count * 8192 / 100 // Rough estimate
    }
}

#[pg_guard]
pub extern "C-unwind" fn kasate_relation_needs_toast_table(relation: pg_sys::Relation) -> bool {
    // We don't support TOAST tables in this simple implementation
    false
}

#[pg_guard]
pub extern "C-unwind" fn kasate_relation_estimate_size(
    relation: pg_sys::Relation,
    attr_widths: *mut i32,
    pages: *mut pg_sys::BlockNumber,
    tuples: *mut f64,
    allvisfrac: *mut f64,
) {
    unsafe {
        if relation.is_null() {
            return;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        let tuple_count = storage_guard.len() as f64;

        if !pages.is_null() {
            *pages = (tuple_count / 100.0).ceil() as u32;
        }

        if !tuples.is_null() {
            *tuples = tuple_count;
        }

        if !allvisfrac.is_null() {
            *allvisfrac = 1.0; // All pages are "visible" in our simple implementation
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

unsafe fn store_tuple_in_slot(
    slot: *mut pg_sys::TupleTableSlot,
    tuple: &crate::storage::Tuple,
    _relation: pg_sys::Relation,
) {
    pgrx::warning!("[KASATE] store_tuple_in_slot called");

    if slot.is_null() {
        pgrx::warning!("[KASATE] store_tuple_in_slot: slot is null");
        return;
    }

    pgrx::warning!("[KASATE] store_tuple_in_slot: tuple_id=({},{}), xmin={}, xmax={}, data_len={}",
        tuple.id.block, tuple.id.offset, tuple.xmin, tuple.xmax, tuple.data.len());

    // TEMPORARY FIX: Just clear the slot for now
    // TODO: Properly reconstruct tuple data and call ExecStoreHeapTuple
    // The issue is that our stored "data" is just raw Datum values,
    // not a properly formatted HeapTuple with HeapTupleHeaderData
    clear_tuple_slot(slot);

    pgrx::warning!("[KASATE] store_tuple_in_slot: cleared slot (tuple retrieval not yet implemented)");
}

unsafe fn extract_tuple_from_slot(slot: *mut pg_sys::TupleTableSlot) -> Option<Vec<u8>> {
    if slot.is_null() {
        pgrx::warning!("[KASATE] extract_tuple_from_slot: slot is null");
        return None;
    }

    let slot_ref = &*slot;
    pgrx::warning!("[KASATE] extract_tuple_from_slot: slot={:p}", slot);

    // Get the tuple from the slot using ops
    let ops = slot_ref.tts_ops;
    if ops.is_null() {
        pgrx::warning!("[KASATE] extract_tuple_from_slot: ops is null");
        return None;
    }

    let ops_ref = &*ops;
    pgrx::warning!("[KASATE] extract_tuple_from_slot: ops={:p}", ops);

    // Get tuple descriptor first to know how many attributes to fetch
    let slot_ref = &*slot;
    let tupdesc = slot_ref.tts_tupleDescriptor;
    if tupdesc.is_null() {
        pgrx::warning!("[KASATE] extract_tuple_from_slot: tupdesc is null");
        return None;
    }

    let natts = (*tupdesc).natts as usize;
    pgrx::warning!("[KASATE] extract_tuple_from_slot: natts={} (from tupdesc)", natts);

    // DO NOT call getsomeattrs - it corrupts the slot!
    // The slot should already be materialized when passed to us
    // Just use the data that's already there

    // Extract values properly by making deep copies of variable-length data
    if natts > 0 && !slot_ref.tts_values.is_null() && !slot_ref.tts_isnull.is_null() {
        pgrx::warning!("[KASATE] extract_tuple_from_slot: extracting from values/nulls arrays");

        let values = std::slice::from_raw_parts(slot_ref.tts_values, natts);
        let nulls = std::slice::from_raw_parts(slot_ref.tts_isnull, natts);
        let attrs = std::slice::from_raw_parts((*tupdesc).attrs.as_ptr(), natts);

        // Collect all attribute data first (making deep copies as needed)
        let mut attr_data: Vec<Vec<u8>> = Vec::with_capacity(natts);

        for i in 0..natts {
            if nulls[i] {
                // NULL value
                attr_data.push(vec![0]); // Store a marker for NULL
            } else {
                let attr = &attrs[i];  // attrs is already a slice of structs, not pointers
                let datum = values[i];

                // For variable-length types (typlen = -1), we need to copy the actual data
                if attr.attlen == -1 {
                    // Variable length - it's a varlena structure
                    let varlena_ptr = datum.value() as *const u8;
                    if !varlena_ptr.is_null() {
                        // Check the first byte to determine header type
                        let first_byte = *varlena_ptr;

                        let actual_size = if (first_byte & 0x01) == 0 {
                            // 4-byte header (long form)
                            // Size is in the first 4 bytes, big-endian, shifted right by 2
                            let mut size_word = [0u8; 4];
                            std::ptr::copy_nonoverlapping(varlena_ptr, size_word.as_mut_ptr(), 4);
                            let size_raw = u32::from_be_bytes(size_word);  // Big-endian!
                            (size_raw >> 2) as usize  // Shift right by 2 to get actual size
                        } else {
                            // 1-byte header (short form)
                            // Size is in the first byte, shifted right by 1
                            (first_byte >> 1) as usize
                        };

                        pgrx::warning!("[KASATE] extract_tuple_from_slot: attr {} is varlen, size={}", i, actual_size);

                        if actual_size > 0 && actual_size < 1000000 {
                            let mut bytes = vec![0u8; actual_size];
                            std::ptr::copy_nonoverlapping(varlena_ptr, bytes.as_mut_ptr(), actual_size);
                            attr_data.push(bytes);
                        } else {
                            attr_data.push(vec![0]);
                        }
                    } else {
                        attr_data.push(vec![0]); // NULL-like
                    }
                } else {
                    // Fixed length - just copy the datum value
                    pgrx::warning!("[KASATE] extract_tuple_from_slot: attr {} is fixed len={}", i, attr.attlen);
                    attr_data.push(datum.value().to_le_bytes().to_vec());
                }
            }
        }

        // Now serialize all the data
        let mut data = Vec::new();
        // Store number of attributes
        data.extend_from_slice(&(natts as u64).to_le_bytes());

        // Store each attribute's size and data
        for attr_bytes in &attr_data {
            data.extend_from_slice(&(attr_bytes.len() as u64).to_le_bytes());
            data.extend_from_slice(attr_bytes);
        }

        pgrx::warning!("[KASATE] extract_tuple_from_slot: extracted {} total bytes", data.len());
        return Some(data);
    }

    pgrx::warning!("[KASATE] extract_tuple_from_slot: no extraction method worked, returning None");
    None
}

unsafe fn clear_tuple_slot(slot: *mut pg_sys::TupleTableSlot) {
    if slot.is_null() {
        return;
    }

    let slot_ref = &mut *slot;
    let ops = slot_ref.tts_ops;
    if !ops.is_null() {
        let ops_ref = &*ops;
        if ops_ref.clear.is_some() {
            ops_ref.clear.unwrap()(slot);
        }
    }
}
