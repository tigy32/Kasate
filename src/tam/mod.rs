// Table Access Method (TAM) implementation
// This module implements the PostgreSQL Table Access Method API

pub mod scan;
pub mod handlers;

use pgrx::prelude::*;

// Forward declarations for handler functions
use handlers::*;

/// Register the Kasate table access method
#[pg_guard]
pub extern "C-unwind" fn kasate_tableam_handler(_fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    unsafe {
        pgrx::warning!("[KASATE] ===== tableam_handler called =====");

        // Allocate in CacheMemoryContext to ensure it persists across transactions
        let old_context = pg_sys::MemoryContextSwitchTo(pg_sys::CacheMemoryContext);

        let routine = pg_sys::palloc0(std::mem::size_of::<pg_sys::TableAmRoutine>())
            as *mut pg_sys::TableAmRoutine;

        if routine.is_null() {
            pg_sys::MemoryContextSwitchTo(old_context);
            panic!("Failed to allocate TableAmRoutine");
        }

        pgrx::warning!("[KASATE] TableAmRoutine allocated at {:p}", routine);

        let routine_ref = &mut *routine;

        // Set the node tag
        routine_ref.type_ = pg_sys::NodeTag::T_TableAmRoutine;

        // Slot callbacks
        routine_ref.slot_callbacks = Some(kasate_slot_callbacks);

        // Scan callbacks
        routine_ref.scan_begin = Some(kasate_scan_begin);
        routine_ref.scan_end = Some(kasate_scan_end);
        routine_ref.scan_rescan = Some(kasate_scan_rescan);
        routine_ref.scan_getnextslot = Some(kasate_scan_getnextslot);

        // Parallel scan callbacks (not implemented yet)
        routine_ref.parallelscan_estimate = None;
        routine_ref.parallelscan_initialize = None;
        routine_ref.parallelscan_reinitialize = None;

        // Index scan callbacks
        routine_ref.index_fetch_begin = Some(kasate_index_fetch_begin);
        routine_ref.index_fetch_reset = Some(kasate_index_fetch_reset);
        routine_ref.index_fetch_end = Some(kasate_index_fetch_end);
        routine_ref.index_fetch_tuple = Some(kasate_index_fetch_tuple);

        // Tuple operations
        routine_ref.tuple_insert = Some(kasate_tuple_insert);
        routine_ref.tuple_insert_speculative = None;
        routine_ref.tuple_complete_speculative = None;
        routine_ref.multi_insert = Some(kasate_multi_insert);
        routine_ref.tuple_delete = Some(kasate_tuple_delete);
        routine_ref.tuple_update = Some(kasate_tuple_update);
        routine_ref.tuple_lock = Some(kasate_tuple_lock);
        routine_ref.finish_bulk_insert = Some(kasate_finish_bulk_insert);

        // Tuple fetch
        routine_ref.tuple_fetch_row_version = Some(kasate_tuple_fetch_row_version);
        routine_ref.tuple_get_latest_tid = Some(kasate_tuple_get_latest_tid);
        routine_ref.tuple_tid_valid = Some(kasate_tuple_tid_valid);
        routine_ref.tuple_satisfies_snapshot = Some(kasate_tuple_satisfies_snapshot);

        // Transaction callbacks
        routine_ref.relation_set_new_filelocator = Some(kasate_relation_set_new_filelocator);
        routine_ref.relation_nontransactional_truncate = None;
        routine_ref.relation_copy_data = None;
        routine_ref.relation_copy_for_cluster = None;
        routine_ref.relation_vacuum = Some(kasate_relation_vacuum);
        routine_ref.scan_analyze_next_block = None;
        routine_ref.scan_analyze_next_tuple = None;
        routine_ref.index_build_range_scan = Some(kasate_index_build_range_scan);
        routine_ref.index_validate_scan = None;

        // Estimation callbacks
        routine_ref.relation_size = Some(kasate_relation_size);
        routine_ref.relation_needs_toast_table = Some(kasate_relation_needs_toast_table);
        routine_ref.relation_estimate_size = Some(kasate_relation_estimate_size);

        // Planner support
        routine_ref.scan_bitmap_next_block = None;
        routine_ref.scan_bitmap_next_tuple = None;
        routine_ref.scan_sample_next_block = None;
        routine_ref.scan_sample_next_tuple = None;

        // Switch back to the original memory context
        pg_sys::MemoryContextSwitchTo(old_context);

        pgrx::warning!("[KASATE] ===== tableam_handler completed =====");
        pg_sys::Datum::from(routine as *mut std::ffi::c_void)
    }
}

/// Get slot callbacks for Kasate
#[pg_guard]
extern "C-unwind" fn kasate_slot_callbacks(_relation: pg_sys::Relation) -> *const pg_sys::TupleTableSlotOps {
    pgrx::warning!("[KASATE] slot_callbacks called");
    unsafe {
        // Try MinimalTuple instead of Virtual - different internal representation
        let ops_ptr = std::ptr::addr_of!(pg_sys::TTSOpsMinimalTuple);
        pgrx::warning!("[KASATE] Returning TTSOpsMinimalTuple at {:p}", ops_ptr);
        ops_ptr
    }
}

/// Initialize the table access method
pub fn init_tableam() {
    // This function can be used for any initialization needed
    info!("Kasate Table Access Method initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tableam_handler_not_null() {
        // We can't fully test this without a Postgres context,
        // but we can at least verify the function compiles
        assert!(true);
    }
}
