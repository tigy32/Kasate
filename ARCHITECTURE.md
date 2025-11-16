# Kasate Architecture

## Overview

Kasate is a PostgreSQL table access method (TAM) that replaces the standard heap storage with a BTreeMap-based in-memory storage. The key architectural principle is **minimizing unsafe code** by bridging from C FFI to safe Rust as quickly as possible.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     PostgreSQL Core                     │
│                  (C code, pg_sys)                       │
└─────────────────┬───────────────────────────────────────┘
                  │ TAM API Calls
                  ▼
┌─────────────────────────────────────────────────────────┐
│              TAM Handler (tam/mod.rs)                   │
│         Registers callbacks with PostgreSQL             │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│          TAM Callbacks (tam/handlers.rs)                │
│   Implements all required TAM API functions             │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│           Bridge Module (bridge/mod.rs)                 │
│    Converts C types → Safe Rust types IMMEDIATELY       │
│              ⚠️ UNSAFE CODE HERE ⚠️                     │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│        Storage Module (storage/mod.rs)                  │
│      BTreeMap-based storage in SAFE RUST               │
│   Uses: Vec, BTreeMap, RwLock, Arc - all safe!         │
└─────────────────────────────────────────────────────────┘
```

## Module Details

### 1. Storage Module (`src/storage/`)

**Purpose**: Provide a safe Rust storage backend using standard collections.

**Key Types**:

```rust
// Safe tuple identifier
pub struct TupleId {
    pub block: u32,
    pub offset: u16,
}

// Safe tuple representation
pub struct Tuple {
    pub id: TupleId,
    pub data: Vec<u8>,      // Safe Vec!
    pub xmin: u32,
    pub xmax: u32,
    pub cid: u32,
}

// Safe relation storage
pub struct RelationStorage {
    tuples: BTreeMap<TupleId, Tuple>,  // Safe BTreeMap!
    next_block: u32,
    next_offset: u16,
}

// Global storage manager
pub struct StorageManager {
    relations: RwLock<BTreeMap<u32, Arc<RwLock<RelationStorage>>>>,
}
```

**Safety Features**:
- No unsafe code in this module
- All data structures are safe Rust
- Concurrency handled with `RwLock` and `Arc`
- Memory management automatic (no manual malloc/free)

**Operations**:
- `insert()`: Add new tuple
- `update()`: Create new version, mark old as deleted
- `delete()`: Mark tuple as deleted (set xmax)
- `scan()`: Iterate over all tuples
- `scan_visible()`: Iterate over visible tuples (MVCC)

### 2. Bridge Module (`src/bridge/`)

**Purpose**: Convert PostgreSQL C types to safe Rust types immediately.

**Key Functions**:

```rust
// Convert Postgres ItemPointer → safe TupleId
pub fn item_pointer_to_tuple_id(ctid: pg_sys::ItemPointerData) -> TupleId

// Extract tuple data → safe Vec<u8>
pub fn extract_tuple_data(tuple: *mut pg_sys::HeapTupleData) -> Option<Vec<u8>>

// Extract transaction IDs
pub fn extract_tuple_xids(tuple: *mut pg_sys::HeapTupleData) -> (u32, u32)

// Create safe Tuple from HeapTuple
pub fn heap_tuple_to_safe_tuple(...) -> Option<Tuple>
```

**Safety Strategy**:
- All unsafe code isolated here
- Minimal unsafe blocks
- Immediate validation of pointers
- Copy data into safe Vec/types ASAP
- No unsafe references escape this module

### 3. TAM Module (`src/tam/`)

**Purpose**: Implement PostgreSQL's Table Access Method API.

#### TAM Handler (`tam/mod.rs`)

Registers all callbacks with PostgreSQL:

```rust
pub extern "C" fn kasate_tableam_handler() -> pg_sys::Datum {
    // Allocate TableAmRoutine
    // Set all callback pointers
    // Return to PostgreSQL
}
```

#### TAM Callbacks (`tam/handlers.rs`)

Implements ~30 TAM API functions:

**Scan Operations**:
- `kasate_scan_begin`: Start sequential scan
- `kasate_scan_getnextslot`: Get next tuple
- `kasate_scan_rescan`: Restart scan
- `kasate_scan_end`: End scan

**Tuple Operations**:
- `kasate_tuple_insert`: Insert new tuple
- `kasate_tuple_update`: Update existing tuple
- `kasate_tuple_delete`: Delete tuple
- `kasate_tuple_lock`: Lock tuple

**Index Operations**:
- `kasate_index_fetch_begin`: Start index scan
- `kasate_index_fetch_tuple`: Fetch tuple by TID
- `kasate_index_build_range_scan`: Build index

**Relation Operations**:
- `kasate_relation_size`: Get relation size
- `kasate_relation_estimate_size`: Estimate size
- `kasate_relation_vacuum`: Vacuum relation

#### Scan Descriptors (`tam/scan.rs`)

Manages scan state in safe Rust:

```rust
pub struct KasateScanDesc {
    pub relation_oid: u32,
    pub snapshot_xmin: u32,
    pub snapshot_xmax: u32,
    pub iterator_state: Option<IteratorState>,
}

pub struct IteratorState {
    pub tuples: Vec<(TupleId, Tuple)>,
    pub current_index: usize,
}
```

## Data Flow Examples

### INSERT Operation

```
1. PostgreSQL calls kasate_tuple_insert(relation, slot, ...)
   ↓
2. Extract relation OID (bridge::extract_relation_oid)
   ↓
3. Extract tuple data from slot → Vec<u8> (bridge::extract_tuple_from_slot)
   ↓
4. Get current XID (bridge::get_current_transaction_id)
   ↓
5. Get storage for relation (STORAGE.get_or_create_relation)
   ↓
6. Insert into BTreeMap (storage.insert(data, xid))
   ↓
7. Update slot with new TID
```

### SELECT (Sequential Scan)

```
1. PostgreSQL calls kasate_scan_begin(relation, snapshot, ...)
   ↓
2. Extract relation OID and snapshot bounds
   ↓
3. Create KasateScanDesc with safe types
   ↓
4. Collect visible tuples into Vec<(TupleId, Tuple)>
   ↓
5. For each kasate_scan_getnextslot call:
   - Get next tuple from Vec
   - Convert Tuple → HeapTuple (allocate with palloc)
   - Store in slot
   - Return to PostgreSQL
```

### UPDATE Operation

```
1. PostgreSQL calls kasate_tuple_update(relation, old_tid, slot, ...)
   ↓
2. Extract old TID → TupleId
   ↓
3. Extract new data from slot → Vec<u8>
   ↓
4. Get current XID
   ↓
5. Call storage.update(old_tid, new_data, xid, xid)
   - Marks old tuple with xmax = xid
   - Creates new tuple with xmin = xid
   - Returns new TupleId
   ↓
6. Update slot with new TID
```

## MVCC Implementation

### Visibility Rules

Each tuple tracks:
- `xmin`: Transaction that inserted it
- `xmax`: Transaction that deleted it (0 if not deleted)

A tuple is visible to a snapshot if:
```rust
tuple.xmin < snapshot.xmax &&  // Inserted before snapshot
(tuple.xmax == 0 || tuple.xmax >= snapshot.xmin)  // Not deleted or deleted after
```

### Transaction Isolation

Simplified implementation:
- Each transaction gets a unique XID
- Snapshots capture range [xmin, xmax)
- Visibility checks use snapshot bounds

**Note**: This is simplified MVCC. Production systems need:
- Transaction status (committed/aborted)
- In-progress transaction tracking
- Vacuum to clean dead tuples

## Memory Management

### PostgreSQL Side (C)

Uses `palloc`/`pfree`:
- `palloc()`: Allocate in current memory context
- Memory freed automatically at context destruction

### Rust Side (Safe)

Automatic memory management:
- `Vec`: Owns its buffer, frees on drop
- `BTreeMap`: Owns nodes, frees on drop
- `Arc`: Reference counted, frees when count = 0
- `Box`: Owns heap allocation, frees on drop

### Bridge Strategy

1. **C → Rust**: Copy data immediately
   ```rust
   let data = Vec::from_raw_parts(c_ptr, len, len);
   ```

2. **Rust → C**: Allocate with palloc, copy data
   ```rust
   let c_ptr = pg_sys::palloc(size);
   ptr::copy_nonoverlapping(rust_data, c_ptr, len);
   ```

## Concurrency

### Read/Write Locking

```rust
pub struct StorageManager {
    relations: RwLock<BTreeMap<...>>,  // Outer lock for relation map
}

pub struct RelationStorage {
    tuples: BTreeMap<...>,  // Protected by per-relation RwLock
}
```

**Benefits**:
- Multiple concurrent readers
- Exclusive writer access
- Per-relation locking (not global)

### Lock Acquisition

```rust
// Read access
let storage = STORAGE.get_or_create_relation(oid);
let guard = storage.read().unwrap();
// ... read operations ...

// Write access
let storage = STORAGE.get_or_create_relation(oid);
let mut guard = storage.write().unwrap();
// ... write operations ...
```

## Testing Strategy

### 1. Unit Tests

Test safe Rust code in isolation:
```rust
#[test]
fn test_tuple_visibility() {
    let tuple = Tuple::new(...);
    assert!(tuple.is_visible(0, 100));
}
```

### 2. PGRX Tests

Test with PostgreSQL context:
```rust
#[pg_test]
fn test_storage_insert() {
    let storage = STORAGE.get_or_create_relation(12345);
    // ...
}
```

### 3. Integration Tests

Test full SQL operations:
```sql
CREATE TABLE test (id int) USING kasate;
INSERT INTO test VALUES (1);
SELECT * FROM test;
```

## Performance Considerations

### Current Implementation

- **Pros**:
  - Simple and easy to understand
  - Safe (no memory corruption)
  - Good for small datasets

- **Cons**:
  - No persistence (in-memory only)
  - Not optimized for large datasets
  - Copies data at FFI boundary

### Potential Optimizations

1. **Zero-copy reads**: Use references instead of clones
2. **Better indexing**: Use more efficient data structures
3. **Lazy materialization**: Don't copy until needed
4. **Batch operations**: Reduce lock overhead
5. **Async I/O**: For disk-based storage

## Limitations

1. **No persistence**: Data lost on restart
2. **No TOAST**: Can't handle very large values
3. **Simplified MVCC**: Not full transaction isolation
4. **No vacuum**: Dead tuples accumulate
5. **No parallel scans**: Single-threaded scans
6. **Memory overhead**: Copies data frequently

## Future Enhancements

### Short Term
- Add basic persistence (serialize to disk)
- Implement vacuum to clean dead tuples
- Better error handling

### Medium Term
- TOAST support for large values
- Parallel scan support
- Better transaction isolation
- Write-ahead logging

### Long Term
- Full ACID compliance
- Performance optimization
- Production-grade reliability
- Comprehensive benchmarks

## References

- [PostgreSQL Table Access Methods](https://www.postgresql.org/docs/16/tableam.html)
- [PGRX Book](https://github.com/pgcentralfoundation/pgrx)
- [Rust Unsafe Code Guidelines](https://rust-lang.github.io/unsafe-code-guidelines/)
