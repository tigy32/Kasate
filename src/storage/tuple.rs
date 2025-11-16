// Safe Rust types for tuple operations

use super::TupleId;

/// Tuple descriptor - describes the structure of a tuple
#[derive(Debug, Clone)]
pub struct TupleDesc {
    pub natts: usize,
    pub attrs: Vec<AttrDesc>,
}

impl TupleDesc {
    pub fn new(natts: usize) -> Self {
        Self {
            natts,
            attrs: Vec::with_capacity(natts),
        }
    }

    pub fn add_attr(&mut self, attr: AttrDesc) {
        self.attrs.push(attr);
    }
}

/// Attribute descriptor
#[derive(Debug, Clone)]
pub struct AttrDesc {
    pub name: String,
    pub typid: u32,
    pub typlen: i16,
    pub typmod: i32,
    pub attnum: i16,
}

impl AttrDesc {
    pub fn new(name: String, typid: u32, typlen: i16, typmod: i32, attnum: i16) -> Self {
        Self {
            name,
            typid,
            typlen,
            typmod,
            attnum,
        }
    }
}

/// Tuple slot - safe representation of a tuple being processed
#[derive(Debug, Clone)]
pub struct TupleSlot {
    pub tid: Option<TupleId>,
    pub data: Option<Vec<u8>>,
    pub desc: Option<TupleDesc>,
}

impl TupleSlot {
    pub fn new() -> Self {
        Self {
            tid: None,
            data: None,
            desc: None,
        }
    }

    pub fn with_tid(tid: TupleId) -> Self {
        Self {
            tid: Some(tid),
            data: None,
            desc: None,
        }
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = Some(data);
    }

    pub fn set_tid(&mut self, tid: TupleId) {
        self.tid = Some(tid);
    }

    pub fn clear(&mut self) {
        self.tid = None;
        self.data = None;
    }

    pub fn is_empty(&self) -> bool {
        self.tid.is_none() && self.data.is_none()
    }
}

impl Default for TupleSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_desc() {
        let mut desc = TupleDesc::new(2);
        desc.add_attr(AttrDesc::new("id".to_string(), 23, 4, -1, 1));
        desc.add_attr(AttrDesc::new("name".to_string(), 25, -1, -1, 2));

        assert_eq!(desc.natts, 2);
        assert_eq!(desc.attrs.len(), 2);
        assert_eq!(desc.attrs[0].name, "id");
    }

    #[test]
    fn test_tuple_slot() {
        let mut slot = TupleSlot::new();
        assert!(slot.is_empty());

        slot.set_tid(TupleId::new(0, 0));
        assert!(!slot.is_empty());

        slot.clear();
        assert!(slot.is_empty());
    }
}
