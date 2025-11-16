// Bridge module - converts from C FFI to safe Rust types as quickly as possible

use pgrx::prelude::*;
use crate::storage::{TupleId, Tuple};

/// Convert Postgres ItemPointer to safe TupleId
pub fn item_pointer_to_tuple_id(ctid: pg_sys::ItemPointerData) -> TupleId {
    unsafe {
        let block = ctid.ip_blkid.bi_hi as u32 * 65536 + ctid.ip_blkid.bi_lo as u32;
        let offset = ctid.ip_posid;
        TupleId::new(block, offset)
    }
}

/// Convert safe TupleId to Postgres ItemPointer
pub fn tuple_id_to_item_pointer(tid: TupleId) -> pg_sys::ItemPointerData {
    let mut ctid = pg_sys::ItemPointerData::default();
    ctid.ip_blkid.bi_hi = (tid.block / 65536) as u16;
    ctid.ip_blkid.bi_lo = (tid.block % 65536) as u16;
    ctid.ip_posid = tid.offset;
    ctid
}

/// Extract tuple data from HeapTuple into safe Vec<u8>
pub fn extract_tuple_data(tuple: *mut pg_sys::HeapTupleData) -> Option<Vec<u8>> {
    if tuple.is_null() {
        return None;
    }

    unsafe {
        let tuple_ref = &*tuple;
        let len = tuple_ref.t_len as usize;
        let data_ptr = tuple_ref.t_data as *const u8;

        if data_ptr.is_null() || len == 0 {
            return None;
        }

        // Copy data into safe Rust Vec
        let mut data = Vec::with_capacity(len);
        std::ptr::copy_nonoverlapping(data_ptr, data.as_mut_ptr(), len);
        data.set_len(len);

        Some(data)
    }
}

/// Extract transaction IDs from HeapTuple
pub fn extract_tuple_xids(tuple: *mut pg_sys::HeapTupleData) -> (u32, u32) {
    if tuple.is_null() {
        return (0, 0);
    }

    unsafe {
        let tuple_ref = &*tuple;
        let header = tuple_ref.t_data;
        if header.is_null() {
            return (0, 0);
        }

        let header_ref = &*header;
        let xmin = header_ref.t_choice.t_heap.t_xmin.into();
        let xmax = header_ref.t_choice.t_heap.t_xmax.into();

        (xmin, xmax)
    }
}

/// Create a safe Tuple from HeapTuple
pub fn heap_tuple_to_safe_tuple(tuple: *mut pg_sys::HeapTupleData, tid: TupleId) -> Option<Tuple> {
    let data = extract_tuple_data(tuple)?;
    let (xmin, xmax) = extract_tuple_xids(tuple);

    let mut safe_tuple = Tuple::new(tid, data, xmin);
    safe_tuple.xmax = xmax;

    Some(safe_tuple)
}

/// Copy safe tuple data back to HeapTuple
pub fn copy_safe_tuple_to_heap(
    safe_tuple: &Tuple,
    tuple: *mut pg_sys::HeapTupleData,
) -> bool {
    if tuple.is_null() {
        return false;
    }

    unsafe {
        let tuple_ref = &mut *tuple;

        // Allocate HeapTupleHeader if needed
        if tuple_ref.t_data.is_null() {
            let size = safe_tuple.data.len();
            let header = pg_sys::palloc(size) as *mut pg_sys::HeapTupleHeaderData;
            if header.is_null() {
                return false;
            }
            tuple_ref.t_data = header;
            tuple_ref.t_len = size as u32;
        }

        // Copy data
        let dest = tuple_ref.t_data as *mut u8;
        let src = safe_tuple.data.as_ptr();
        let len = safe_tuple.data.len().min(tuple_ref.t_len as usize);
        std::ptr::copy_nonoverlapping(src, dest, len);

        // Set transaction IDs
        let header_ref = &mut *tuple_ref.t_data;
        header_ref.t_choice.t_heap.t_xmin = safe_tuple.xmin.into();
        header_ref.t_choice.t_heap.t_xmax = safe_tuple.xmax.into();

        true
    }
}

/// Extract relation OID from Relation
pub fn extract_relation_oid(relation: pg_sys::Relation) -> Option<u32> {
    if relation.is_null() {
        return None;
    }

    unsafe {
        let rel_ref = &*relation;
        Some(rel_ref.rd_id.into())
    }
}

/// Extract current transaction ID
pub fn get_current_transaction_id() -> u32 {
    unsafe {
        pg_sys::GetCurrentTransactionId().into()
    }
}

/// Extract snapshot information
pub fn extract_snapshot_info(snapshot: pg_sys::Snapshot) -> (u32, u32) {
    if snapshot.is_null() {
        // Return conservative snapshot
        return (0, u32::MAX);
    }

    unsafe {
        let snap_ref = &*snapshot;
        let xmin = snap_ref.xmin.into();
        let xmax = snap_ref.xmax.into();
        (xmin, xmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_id_conversion() {
        let tid = TupleId::new(42, 10);
        let item_ptr = tuple_id_to_item_pointer(tid);
        let tid_back = item_pointer_to_tuple_id(item_ptr);

        assert_eq!(tid, tid_back);
    }
}
