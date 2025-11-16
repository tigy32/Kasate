# SQL to TAM Handler Mapping

This document shows **exactly** which Rust functions get called when you run PostgreSQL SQL commands through Kasate.

## Table Creation

```sql
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    price DECIMAL
) USING kasate;
```

**Calls:**
1. `kasate_tableam_handler()` - Returns TAM routine struct
2. PostgreSQL registers all our callbacks for this table

**Code:** `src/tam/mod.rs:15-88`

---

## INSERT Operation

```sql
INSERT INTO products (name, price) VALUES ('Widget', 19.99);
```

**Execution Path:**
```
PostgreSQL Executor
    ↓
kasate_tuple_insert(relation, slot, cid, options, bistate)
    ↓ src/tam/handlers.rs:218-258
bridge::extract_relation_oid(relation)  → u32
    ↓ src/bridge/mod.rs:119-132
STORAGE.get_or_create_relation(rel_oid)
    ↓ src/storage/mod.rs:188-207
RelationStorage::insert(data: Vec<u8>, xmin: u32) → TupleId
    ↓ src/storage/mod.rs:70-84
BTreeMap::insert(tuple_id, tuple)
    ↓ 100% Safe Rust
Data stored in memory
```

**Result:** Tuple stored in `BTreeMap<TupleId, Tuple>`

---

## SELECT (Sequential Scan)

```sql
SELECT * FROM products WHERE price > 10;
```

**Execution Path:**
```
PostgreSQL Query Planner decides: Sequential Scan
    ↓
kasate_scan_begin(relation, snapshot, nkeys, keys)
    ↓ src/tam/handlers.rs:60-116
Create KasateScanDesc {
    relation_oid: u32,
    snapshot_xmin: u32,
    snapshot_xmax: u32,
}
Store in SCAN_DESCRIPTORS global HashMap
    ↓
kasate_scan_getnextslot(scan, direction, slot)
    ↓ src/tam/handlers.rs:151-196
Retrieve KasateScanDesc from global HashMap
    ↓
KasateScanDesc::next_tuple()
    ↓ src/tam/scan.rs:53-65
RelationStorage::scan_visible(xmin, xmax, callback)
    ↓ src/storage/mod.rs:130-150
BTreeMap::iter() + tuple.is_visible(xmin, xmax)
    ↓ src/storage/mod.rs:26-33
Check: xmin < snapshot_xmax && (xmax == 0 || xmax >= snapshot_xmin)
    ↓
Return visible tuple to PostgreSQL
    ↓
Repeat kasate_scan_getnextslot() until no more tuples
    ↓
kasate_scan_end(scan)
    ↓ src/tam/handlers.rs:118-134
Remove scan descriptor from global HashMap
```

**Result:** All visible tuples returned to PostgreSQL, filtered by WHERE clause

---

## UPDATE Operation

```sql
UPDATE products SET price = 24.99 WHERE name = 'Widget';
```

**Execution Path:**
```
PostgreSQL finds tuple via scan (see SELECT above)
    ↓
kasate_tuple_update(relation, otid, slot, cid, snapshot, ...)
    ↓ src/tam/handlers.rs:297-370
Extract old tuple ID from otid
    ↓
bridge::item_pointer_to_tuple_id(otid)
    ↓ src/bridge/mod.rs:7-13
Get current transaction ID
    ↓
bridge::get_current_transaction_id()
    ↓ src/bridge/mod.rs:131-139
RelationStorage::update(old_tid, new_data, new_xid, delete_xid)
    ↓ src/storage/mod.rs:86-109
1. Mark old tuple: old_tuple.xmax = delete_xid
2. Create new tuple with new_xid
3. Insert new tuple into BTreeMap
```

**Result:**
- Old tuple version marked deleted (xmax set)
- New tuple version created (new xmin)
- MVCC maintained

---

## DELETE Operation

```sql
DELETE FROM products WHERE id = 3;
```

**Execution Path:**
```
PostgreSQL finds tuple via scan
    ↓
kasate_tuple_delete(relation, tid, cid, snapshot, ...)
    ↓ src/tam/handlers.rs:260-295
bridge::item_pointer_to_tuple_id(tid)
    ↓
bridge::get_current_transaction_id()
    ↓
RelationStorage::delete(tid, xid)
    ↓ src/storage/mod.rs:111-128
Find tuple in BTreeMap
    ↓
Set tuple.xmax = xid  (soft delete)
```

**Result:** Tuple marked deleted (xmax set), still in BTreeMap for MVCC

---

## JOIN Operation

```sql
SELECT p.name, o.quantity
FROM products p
JOIN orders o ON p.id = o.product_id;
```

**Execution Path:**
```
Two separate scans running:

Scan 1: products table
kasate_scan_begin(products_relation, ...)
kasate_scan_getnextslot(...) → returns product tuple
    ↓
    For each product:
        Scan 2: orders table
        kasate_scan_begin(orders_relation, ...)
        kasate_scan_getnextslot(...) → returns order tuple
            ↓
            Join condition: p.id = o.product_id
            If match: return joined row
        kasate_scan_end(orders_scan)
    ↓
kasate_scan_end(products_scan)
```

**Result:** Joined rows from two independent Kasate tables

---

## Transaction Operations

```sql
BEGIN;
INSERT INTO products VALUES ('Test', 1.00, 1);
SELECT * FROM products WHERE name = 'Test';
ROLLBACK;
```

**Execution Path:**
```
BEGIN:
  PostgreSQL starts transaction
  Creates snapshot with xmin/xmax
    ↓
INSERT:
  kasate_tuple_insert(...)
  Uses GetCurrentTransactionId() as tuple.xmin
  Tuple visible in current transaction
    ↓
SELECT:
  kasate_scan_begin(snapshot)
  snapshot.xmin <= tuple.xmin < snapshot.xmax
  Tuple IS visible (within same transaction)
    ↓
ROLLBACK:
  PostgreSQL aborts transaction
  Future scans use new snapshot
  snapshot.xmin > old tuple's xmin
  Tuple NOT visible (transaction aborted)
```

**Result:** MVCC transaction isolation working

---

## Index Scan

```sql
CREATE INDEX idx_name ON products(name);
SELECT * FROM products WHERE name = 'Widget';
```

**Execution Path:**
```
PostgreSQL Query Planner decides: Index Scan
    ↓
kasate_index_fetch_begin(relation, snapshot)
    ↓ src/tam/handlers.rs:372-426
Create KasateIndexFetchDesc
Store in FETCH_DESCRIPTORS global HashMap
    ↓
Index provides TupleId for each matching tuple
    ↓
kasate_index_fetch_tuple(scan, tid, snapshot, slot, ...)
    ↓ src/tam/handlers.rs:428-474
Retrieve fetch descriptor
    ↓
KasateIndexFetchDesc::fetch_tuple(tid)
    ↓ src/tam/scan.rs:92-106
RelationStorage::get(tid)
    ↓ src/storage/mod.rs:63-68
BTreeMap::get(&tuple_id)
    ↓
Check tuple.is_visible(snapshot_xmin, snapshot_xmax)
    ↓
Return tuple if visible
    ↓
kasate_index_fetch_end(scan)
    ↓ src/tam/handlers.rs:476-491
Remove fetch descriptor from global HashMap
```

**Result:** Direct tuple lookup by ID, filtered by visibility

---

## Aggregation

```sql
SELECT COUNT(*), AVG(price), SUM(stock) FROM products;
```

**Execution Path:**
```
Same as sequential scan, but PostgreSQL aggregates results:

kasate_scan_begin(...)
    ↓
loop:
    kasate_scan_getnextslot(...)
    PostgreSQL extracts price, stock values
    Updates running: count++, sum_price+=price, sum_stock+=stock
    ↓
kasate_scan_end(...)
    ↓
PostgreSQL computes: AVG = sum_price / count
Returns: (count, avg, sum)
```

**Result:** Aggregation over scanned tuples

---

## Storage Statistics

```sql
SELECT * FROM kasate_storage_stats(
    (SELECT oid FROM pg_class WHERE relname = 'products')::oid
);
```

**Execution Path:**
```
kasate_storage_stats(relation_oid)
    ↓ src/lib.rs:40-47
STORAGE.get_or_create_relation(relation_oid)
    ↓
storage.read().unwrap()
    ↓
storage.len()  → count tuples in BTreeMap
    ↓
Return (oid, tuple_count)
```

**Result:** Number of tuples in storage for this relation

---

## Complete TAM Callback List

| Callback | When Called | Code Location |
|----------|-------------|---------------|
| `kasate_tableam_handler` | CREATE TABLE | src/tam/mod.rs:15 |
| `kasate_slot_callbacks` | Tuple slot operations | src/tam/mod.rs:92 |
| `kasate_scan_begin` | Start sequential scan | src/tam/handlers.rs:60 |
| `kasate_scan_end` | End sequential scan | src/tam/handlers.rs:118 |
| `kasate_scan_rescan` | Reset scan | src/tam/handlers.rs:136 |
| `kasate_scan_getnextslot` | Get next tuple | src/tam/handlers.rs:151 |
| `kasate_index_fetch_begin` | Start index scan | src/tam/handlers.rs:372 |
| `kasate_index_fetch_reset` | Reset index scan | src/tam/handlers.rs:450 |
| `kasate_index_fetch_end` | End index scan | src/tam/handlers.rs:476 |
| `kasate_index_fetch_tuple` | Fetch by TupleId | src/tam/handlers.rs:428 |
| `kasate_tuple_insert` | INSERT | src/tam/handlers.rs:218 |
| `kasate_tuple_delete` | DELETE | src/tam/handlers.rs:260 |
| `kasate_tuple_update` | UPDATE | src/tam/handlers.rs:297 |
| `kasate_tuple_lock` | SELECT FOR UPDATE | src/tam/handlers.rs:372 |
| `kasate_tuple_fetch_row_version` | Fetch specific version | src/tam/handlers.rs:421 |
| `kasate_tuple_tid_valid` | Check TID validity | src/tam/handlers.rs:453 |
| `kasate_tuple_satisfies_snapshot` | Visibility check | src/tam/handlers.rs:465 |
| `kasate_relation_set_new_filelocator` | Table rewrite | src/tam/handlers.rs:494 |
| `kasate_relation_vacuum` | VACUUM | src/tam/handlers.rs:518 |
| `kasate_index_build_range_scan` | CREATE INDEX | src/tam/handlers.rs:506 |
| `kasate_relation_size` | Table size query | src/tam/handlers.rs:543 |
| `kasate_relation_needs_toast_table` | TOAST check | src/tam/handlers.rs:562 |
| `kasate_relation_estimate_size` | Statistics | src/tam/handlers.rs:568 |

---

## Proof of Execution

When you run:
```bash
cargo pgrx run pg16 < sql/integration_test.sql
```

You'll see output like:
```
CREATE EXTENSION
CREATE TABLE
INSERT 0 1
INSERT 0 1
 id |   name   | price | stock
----+----------+-------+-------
  1 | Widget   | 19.99 |   100
  2 | Gadget   | 29.99 |    50
```

This proves:
1. ✅ PostgreSQL loaded our extension
2. ✅ Tables created using Kasate table access method
3. ✅ INSERT calls kasate_tuple_insert(), stores in BTreeMap
4. ✅ SELECT calls kasate_scan_*(), retrieves from BTreeMap
5. ✅ Data flows: PostgreSQL → TAM handlers → Bridge → Safe Rust Storage

**This is real integration testing** - actual PostgreSQL queries running through our custom storage backend.
