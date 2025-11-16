# Contributing to Kasate

## Development Setup

### Prerequisites

1. Install Rust (1.70 or later):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install PostgreSQL 16:
   - **Ubuntu/Debian**: `sudo apt install postgresql-16 postgresql-server-dev-16`
   - **macOS**: `brew install postgresql@16`
   - **From source**: See [PostgreSQL documentation](https://www.postgresql.org/docs/16/installation.html)

3. Install cargo-pgrx:
   ```bash
   cargo install --locked cargo-pgrx
   cargo pgrx init --pg16 /path/to/pg_config
   ```

### Building

```bash
# Quick build
./build.sh

# Or manually
cargo pgrx build --pg16
```

### Testing

```bash
# Run all tests
./run_tests.sh

# Or manually
cargo test --lib              # Unit tests
cargo pgrx test pg16          # PGRX integration tests
```

### Installing Locally

```bash
cargo pgrx install --pg16
```

Then in PostgreSQL:
```sql
CREATE EXTENSION kasate;
```

## Code Structure

```
src/
├── lib.rs              # Main entry point, PGRX initialization
├── storage/            # Safe Rust storage implementation
│   ├── mod.rs         # BTreeMap-based storage, core types
│   ├── tuple.rs       # Tuple descriptor and slot types
│   ├── relation.rs    # Relation metadata
│   └── transaction.rs # Transaction and snapshot types
├── bridge/             # C FFI to safe Rust conversion
│   └── mod.rs         # Bridge functions
└── tam/                # Table Access Method implementation
    ├── mod.rs         # TAM handler registration
    ├── scan.rs        # Scan descriptors
    └── handlers.rs    # TAM callback implementations
```

## Design Principles

### 1. Minimize Unsafe Code

- **Bridge quickly**: Convert from C FFI to safe Rust types ASAP
- **Isolate unsafe**: All unsafe code should be in the `bridge/` module
- **Safe storage**: The storage layer uses only safe Rust (Vec, BTreeMap, RwLock)

### 2. Clear Separation of Concerns

- **Storage**: Pure Rust data structures, no Postgres dependencies
- **Bridge**: Converts between Postgres C types and our safe types
- **TAM**: Implements Postgres callbacks, delegates to safe storage

### 3. Type Safety

- Define custom types (TupleId, Tuple, etc.) rather than using primitives
- Use the type system to prevent errors at compile time
- Avoid stringly-typed or weakly-typed interfaces

## Code Style

- Follow Rust standard style (use `rustfmt`)
- Document all public APIs
- Add tests for new functionality
- Keep functions focused and small

## Testing Guidelines

### Unit Tests

Add tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        // Test code here
    }
}
```

### PGRX Tests

Add tests in `src/lib.rs`:

```rust
#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_postgres_feature() {
        // Test code here
    }
}
```

### Integration Tests

Add SQL tests to `test_integration.sql` or `examples/`.

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run tests: `./run_tests.sh`
6. Commit with clear messages
7. Push and create a pull request

## Areas for Improvement

This is an educational project, but here are areas where contributions would be valuable:

1. **Persistence**: Add disk-based storage
2. **TOAST support**: Implement TOAST for large values
3. **Better MVCC**: Improve transaction isolation
4. **Vacuum**: Implement proper dead tuple cleanup
5. **Parallel scans**: Add parallel scan support
6. **Performance**: Optimize hot paths
7. **More tests**: Increase test coverage
8. **Documentation**: Improve inline documentation

## Questions?

Open an issue on GitHub!
