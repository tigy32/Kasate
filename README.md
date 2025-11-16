# Kasate

A PostgreSQL table access method implementation using PGRX that replaces the heap storage with a BTreeMap-based storage backend written in safe Rust.

## Features

- **Safe Rust Implementation**: Uses BTreeMap for storage with minimal unsafe code
- **Full Table Access Method Support**: Implements PostgreSQL's TAM API
- **MVCC Support**: Transaction visibility and snapshot isolation
- **Complete CRUD Operations**: Insert, update, delete, and select operations
- **Index Support**: Compatible with PostgreSQL indexes
- **Comprehensive Testing**: Unit tests and integration tests included

## Architecture

### Safe Rust Storage Layer

The extension is designed to bridge from C FFI to safe Rust as quickly as possible:

```
C FFI → Bridge Module → Safe Rust Storage (BTreeMap)
```

Key components:

1. **Storage Module** (`src/storage/`): Safe Rust types and BTreeMap-based storage
   - `TupleId`: Safe tuple identifier
   - `Tuple`: Safe tuple representation with transaction info
   - `RelationStorage`: BTreeMap-based storage for a single relation
   - `StorageManager`: Global manager for all relations

2. **Bridge Module** (`src/bridge/`): Converts C types to safe Rust types
   - Minimal unsafe code to extract data from Postgres C structures
   - Immediate conversion to safe Rust types

3. **TAM Module** (`src/tam/`): Table Access Method implementation
   - Implements all required TAM callbacks
   - Delegates to safe Rust storage layer

## Building

### Prerequisites

- Rust 1.70 or later
- PostgreSQL 16
- PGRX 0.11.4

### Install PGRX

```bash
cargo install --locked cargo-pgrx
cargo pgrx init --pg16 /path/to/pg16
```

### Build the extension

```bash
cargo pgrx build --pg16
```

### Install the extension

```bash
cargo pgrx install --pg16
```

## Usage

### Enable the extension

```sql
CREATE EXTENSION kasate;
```

### Create a table using Kasate

```sql
CREATE TABLE my_table (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    value INTEGER
) USING kasate;
```

### Convert an existing table to Kasate

**Warning**: This will lose all existing data!

```sql
ALTER TABLE existing_table SET ACCESS METHOD kasate;
```

### Check if a table uses Kasate

```sql
SELECT is_using_kasate('my_table');
```

### Get storage statistics

```sql
SELECT * FROM kasate_storage_stats('my_table'::regclass);
```

## Testing

### Run unit tests

```bash
cargo test
```

### Run PGRX tests

```bash
cargo pgrx test pg16
```

### Run integration tests

```bash
psql -d your_database -f test_integration.sql
```

## Implementation Details

### Storage Backend

The storage backend uses a `BTreeMap<TupleId, Tuple>` for each relation, where:

- `TupleId`: Contains block number and offset (mimics PostgreSQL's ItemPointer)
- `Tuple`: Contains tuple data, transaction IDs (xmin, xmax), and command ID

### Transaction Support

Simplified MVCC implementation:

- Each tuple tracks `xmin` (inserting transaction) and `xmax` (deleting transaction)
- Visibility checks compare tuple XIDs against snapshot bounds
- Updates create new tuple versions and mark old versions as deleted

### Concurrency

- Uses `RwLock` for concurrent access to storage
- Read operations can run in parallel
- Write operations require exclusive access

## Limitations

This is a demonstration/educational implementation with several limitations:

1. **Not Production Ready**: This is a proof of concept
2. **No Persistence**: Data is stored in memory only
3. **No TOAST**: Does not support PostgreSQL's TOAST mechanism
4. **Simplified Transactions**: Basic MVCC, no full transaction isolation
5. **No Parallel Scans**: Parallel scan operations not implemented
6. **No Vacuum**: Dead tuples are not automatically cleaned up
7. **Performance**: Not optimized for production workloads

## Safety

The extension prioritizes safety:

- Most code is safe Rust
- Unsafe blocks are minimal and isolated in the bridge module
- All data structures use safe Rust types (Vec, BTreeMap, RwLock)
- No manual memory management in the storage layer

## Contributing

This is an educational project demonstrating:

- How to implement PostgreSQL table access methods
- How to use PGRX effectively
- How to minimize unsafe Rust in PostgreSQL extensions
- How to bridge from C FFI to safe Rust

## License

MIT OR Apache-2.0

## References

- [PostgreSQL Table Access Methods](https://www.postgresql.org/docs/16/tableam.html)
- [PGRX Documentation](https://github.com/pgcentralfoundation/pgrx)
- [PostgreSQL Extension Development](https://www.postgresql.org/docs/16/extend.html)