// TAM handler implementations

use pgrx::prelude::*;
use crate::storage::{TupleId, STORAGE};
use crate::bridge;
use crate::tam::scan::{KasateScanDesc, KasateIndexFetchDesc};
use std::ptr;

// ============================================================================
// Scan callbacks
// ============================================================================

#[pg_guard]
pub extern "C" fn kasate_scan_begin(
    relation: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    nkeys: std::os::raw::c_int,
    key: *mut pg_sys::ScanKeyData,
    _pscan: pg_sys::ParallelTableScanDesc,
    flags: u32,
) -> pg_sys::TableScanDesc {
    unsafe {
        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
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

            // Store our scan descriptor in the opaque field
            pg_scan_ref.rs_scan_opaque = Box::into_raw(scan_desc) as *mut std::ffi::c_void;
        }

        pg_scan
    }
}

#[pg_guard]
pub extern "C" fn kasate_scan_end(scan: pg_sys::TableScanDesc) {
    unsafe {
        if !scan.is_null() {
            let scan_ref = &*scan;
            if !scan_ref.rs_scan_opaque.is_null() {
                // Reclaim our scan descriptor
                let _scan_desc = Box::from_raw(
                    scan_ref.rs_scan_opaque as *mut KasateScanDesc
                );
                // It will be dropped here
            }
        }
    }
}

#[pg_guard]
pub extern "C" fn kasate_scan_rescan(
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

            if !scan_ref.rs_scan_opaque.is_null() {
                let scan_desc = &mut *(scan_ref.rs_scan_opaque as *mut KasateScanDesc);
                scan_desc.rescan();
            }
        }
    }
}

#[pg_guard]
pub extern "C" fn kasate_scan_getnextslot(
    scan: pg_sys::TableScanDesc,
    _direction: pg_sys::ScanDirection::Type,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    unsafe {
        if scan.is_null() || slot.is_null() {
            return false;
        }

        let scan_ref = &mut *scan;
        if scan_ref.rs_scan_opaque.is_null() {
            return false;
        }

        let scan_desc = &mut *(scan_ref.rs_scan_opaque as *mut KasateScanDesc);

        // Get next tuple from our scan
        if let Some(tuple) = scan_desc.next_tuple() {
            // Store tuple in slot
            store_tuple_in_slot(slot, tuple, scan_ref.rs_rd);
            true
        } else {
            // No more tuples
            pg_sys::ExecClearTuple(slot);
            false
        }
    }
}

// ============================================================================
// Index fetch callbacks
// ============================================================================

#[pg_guard]
pub extern "C" fn kasate_index_fetch_begin(
    relation: pg_sys::Relation,
) -> *mut pg_sys::IndexFetchTableData {
    unsafe {
        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);

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
            pg_fetch_ref.opaque = Box::into_raw(fetch_desc) as *mut std::ffi::c_void;
        }

        pg_fetch
    }
}

#[pg_guard]
pub extern "C" fn kasate_index_fetch_reset(fetch: *mut pg_sys::IndexFetchTableData) {
    // Nothing to reset in our implementation
    if fetch.is_null() {
        return;
    }
}

#[pg_guard]
pub extern "C" fn kasate_index_fetch_end(fetch: *mut pg_sys::IndexFetchTableData) {
    unsafe {
        if !fetch.is_null() {
            let fetch_ref = &*fetch;
            if !fetch_ref.opaque.is_null() {
                let _fetch_desc = Box::from_raw(
                    fetch_ref.opaque as *mut KasateIndexFetchDesc
                );
                // It will be dropped here
            }
        }
    }
}

#[pg_guard]
pub extern "C" fn kasate_index_fetch_tuple(
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
        if fetch_ref.opaque.is_null() {
            return false;
        }

        let fetch_desc = &*(fetch_ref.opaque as *const KasateIndexFetchDesc);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);

        if let Some(tuple) = fetch_desc.fetch_tuple(tuple_id) {
            store_tuple_in_slot(slot, &tuple, fetch_ref.rel);
            true
        } else {
            pg_sys::ExecClearTuple(slot);
            false
        }
    }
}

// ============================================================================
// Tuple modification callbacks
// ============================================================================

#[pg_guard]
pub extern "C" fn kasate_tuple_insert(
    relation: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
    _cid: pg_sys::CommandId,
    _options: std::os::raw::c_int,
    _bistate: *mut pg_sys::BulkInsertStateData,
) {
    unsafe {
        if relation.is_null() || slot.is_null() {
            return;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let xid = bridge::get_current_transaction_id();

        // Extract tuple data from slot
        if let Some(tuple_data) = extract_tuple_from_slot(slot) {
            // Insert into storage
            let storage = STORAGE.get_or_create_relation(relation_oid);
            let mut storage_guard = storage.write().unwrap();
            let tid = storage_guard.insert(tuple_data, xid);

            // Update slot with the new TID
            let slot_ref = &mut *slot;
            let item_ptr = bridge::tuple_id_to_item_pointer(tid);
            slot_ref.tts_tid = item_ptr;
        }
    }
}

#[pg_guard]
pub extern "C" fn kasate_tuple_delete(
    relation: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    _cid: pg_sys::CommandId,
    snapshot: pg_sys::Snapshot,
    _crosscheck: pg_sys::Snapshot,
    _wait: bool,
    _tmfd: *mut pg_sys::TM_FailureData,
    _changingPart: bool,
) -> pg_sys::TM_Result::Type {
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
pub extern "C" fn kasate_tuple_update(
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
pub extern "C" fn kasate_tuple_lock(
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
pub extern "C" fn kasate_tuple_fetch_row_version(
    relation: pg_sys::Relation,
    tid: pg_sys::ItemPointer,
    snapshot: pg_sys::Snapshot,
    slot: *mut pg_sys::TupleTableSlot,
) -> bool {
    unsafe {
        if relation.is_null() || tid.is_null() || slot.is_null() {
            return false;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let tuple_id = bridge::item_pointer_to_tuple_id(*tid);
        let (snapshot_xmin, snapshot_xmax) = bridge::extract_snapshot_info(snapshot);

        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        if let Some(tuple) = storage_guard.get(tuple_id) {
            if tuple.is_visible(snapshot_xmin, snapshot_xmax) {
                store_tuple_in_slot(slot, tuple, relation);
                true
            } else {
                pg_sys::ExecClearTuple(slot);
                false
            }
        } else {
            pg_sys::ExecClearTuple(slot);
            false
        }
    }
}

#[pg_guard]
pub extern "C" fn kasate_tuple_tid_valid(
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
pub extern "C" fn kasate_tuple_satisfies_snapshot(
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
pub extern "C" fn kasate_relation_set_new_filelocator(
    relation: pg_sys::Relation,
    _newrlocator: *const pg_sys::RelFileLocator,
    _persistence: ::std::os::raw::c_char,
    _freeze_xid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
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
pub extern "C" fn kasate_relation_vacuum(
    relation: pg_sys::Relation,
    _params: *mut pg_sys::VacuumParams,
    _bstrategy: pg_sys::BufferAccessStrategy,
) {
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
pub extern "C" fn kasate_index_build_range_scan(
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
pub extern "C" fn kasate_relation_size(
    relation: pg_sys::Relation,
    _forkNumber: pg_sys::ForkNumber::Type,
) -> u64 {
    unsafe {
        if relation.is_null() {
            return 0;
        }

        let relation_oid = bridge::extract_relation_oid(relation).unwrap_or(0);
        let storage = STORAGE.get_or_create_relation(relation_oid);
        let storage_guard = storage.read().unwrap();

        // Estimate size based on number of tuples
        let tuple_count = storage_guard.len() as u64;
        tuple_count * 8192 / 100 // Rough estimate
    }
}

#[pg_guard]
pub extern "C" fn kasate_relation_needs_toast_table(relation: pg_sys::Relation) -> bool {
    // We don't support TOAST tables in this simple implementation
    false
}

#[pg_guard]
pub extern "C" fn kasate_relation_estimate_size(
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
    relation: pg_sys::Relation,
) {
    if slot.is_null() {
        return;
    }

    let slot_ref = &mut *slot;

    // Create a HeapTuple from our safe tuple
    let tuple_size = tuple.data.len();
    let heap_tuple = pg_sys::palloc0(
        std::mem::size_of::<pg_sys::HeapTupleData>() + tuple_size
    ) as *mut pg_sys::HeapTupleData;

    if !heap_tuple.is_null() {
        let heap_tuple_ref = &mut *heap_tuple;
        heap_tuple_ref.t_len = tuple_size as u32;

        // Allocate and copy data
        let data_ptr = pg_sys::palloc(tuple_size) as *mut pg_sys::HeapTupleHeaderData;
        if !data_ptr.is_null() {
            std::ptr::copy_nonoverlapping(
                tuple.data.as_ptr(),
                data_ptr as *mut u8,
                tuple_size,
            );
            heap_tuple_ref.t_data = data_ptr;

            // Set transaction info
            let header_ref = &mut *data_ptr;
            header_ref.t_choice.t_heap.t_xmin = tuple.xmin;
            header_ref.t_choice.t_heap.t_xmax = tuple.xmax;
        }

        heap_tuple_ref.t_self = bridge::tuple_id_to_item_pointer(tuple.id);
        heap_tuple_ref.t_tableOid = bridge::extract_relation_oid(relation).unwrap_or(0);

        // Store in slot
        pg_sys::ExecStoreHeapTuple(heap_tuple, slot, false);
    }
}

unsafe fn extract_tuple_from_slot(slot: *mut pg_sys::TupleTableSlot) -> Option<Vec<u8>> {
    if slot.is_null() {
        return None;
    }

    // Materialize the slot if needed
    pg_sys::ExecMaterializeSlot(slot);

    let slot_ref = &*slot;
    let heap_tuple = slot_ref.tts_tuple;

    if heap_tuple.is_null() {
        return None;
    }

    bridge::extract_tuple_data(heap_tuple)
}
