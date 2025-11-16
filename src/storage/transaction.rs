// Safe Rust types for transaction management

use std::sync::atomic::{AtomicU32, Ordering};

/// Transaction snapshot - safe representation
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub xmin: u32,
    pub xmax: u32,
    pub xcnt: u32,
}

impl Snapshot {
    pub fn new(xmin: u32, xmax: u32) -> Self {
        Self {
            xmin,
            xmax,
            xcnt: 0,
        }
    }

    pub fn is_visible(&self, xid: u32) -> bool {
        xid < self.xmax && xid >= self.xmin
    }
}

/// Transaction ID allocator
pub struct TransactionIdAllocator {
    next_xid: AtomicU32,
}

impl TransactionIdAllocator {
    pub fn new() -> Self {
        Self {
            next_xid: AtomicU32::new(1),
        }
    }

    pub fn allocate(&self) -> u32 {
        self.next_xid.fetch_add(1, Ordering::SeqCst)
    }

    pub fn current(&self) -> u32 {
        self.next_xid.load(Ordering::SeqCst)
    }
}

impl Default for TransactionIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// Global transaction ID allocator
lazy_static::lazy_static! {
    pub static ref XID_ALLOCATOR: TransactionIdAllocator = TransactionIdAllocator::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_visibility() {
        let snapshot = Snapshot::new(100, 200);

        assert!(snapshot.is_visible(150));
        assert!(!snapshot.is_visible(50));
        assert!(!snapshot.is_visible(250));
    }

    #[test]
    fn test_xid_allocator() {
        let allocator = TransactionIdAllocator::new();

        let xid1 = allocator.allocate();
        let xid2 = allocator.allocate();

        assert_eq!(xid2, xid1 + 1);
    }
}
