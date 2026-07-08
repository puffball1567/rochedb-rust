# RocheDB Rust Driver

Rust driver for [RocheDB](https://github.com/puffball1567/rochedb).

This crate currently wraps the RocheDB C ABI. It gives Rust applications a safe
embedded API while RocheDB keeps placement, ring metadata, retrieval planning,
and ID generation inside the database core.

## Install

After the crate is published:

```bash
cargo add rochedb
```

Or use RocheDB's driver discovery command from the core CLI:

```bash
roche driver install rust --manifest-path=/path/to/Cargo.toml
```

## Link RocheDB Core

Build the RocheDB shared library first:

```bash
git clone https://github.com/puffball1567/rochedb.git
cd rochedb
nim c --app:lib -d:release --nimcache:/tmp/nimcache_roche_capi -o:lib/librochedb.so src/rochedb_capi.nim
```

Then point this Rust crate at the RocheDB core checkout or shared-library
directory:

```bash
ROCHEDB_CORE_DIR=/path/to/rochedb cargo test
```

or:

```bash
ROCHEDB_LIB_DIR=/path/to/rochedb/lib cargo test
```

If this repository is checked out next to `rochedb` or `ceresdb`, the build
script also detects `../rochedb/lib` and `../ceresdb/lib` automatically.

## Example

```rust
use rochedb::{RetrieveOptions, RocheDb};

let db = RocheDb::open_default()?;
db.set_galaxy_description("Product and support knowledge")?;
db.set_ring_description("docs", "Documentation ring")?;
let id = db.put_vec("docs", br#"{"title":"hello"}"#, &[1.0, 0.0])?;
let value = db.get_string(id)?.unwrap();
let selected = db.query_string(id, "{ title }")?;
let results = db.retrieve_with(
    &[1.0, 0.0],
    RetrieveOptions::new().ring("docs").budget(8),
)?;
let atlas = db.atlas(Some(&[1.0, 0.0]), 8)?;
# Ok::<(), rochedb::Error>(())
```

Run the full example:

```bash
ROCHEDB_CORE_DIR=/path/to/rochedb cargo run --example embedded
```

## Current API Coverage

| Area | Status |
|---|---|
| Embedded open | `RocheDb::open_default`, `open`, `open_dir` |
| Cluster connect | `connect`, `connect_auth`, `ConnectOptions` |
| Writes | `put`, `put_str`, `put_json`, `put_vec` |
| Reads | `get`, `get_string`, `batch_get` |
| Projection | `query`, `query_string` |
| Retrieval | `retrieve`, `retrieve_with`, `RetrieveOptions` |
| Atlas | `atlas` |
| Ring / galaxy metadata | `configure_ring`, `set_galaxy_description`, `set_ring_description` |
| Orbit helpers | `now`, `advance`, `locate`, `next_visit`, `next_join` |
| Error handling | `Result<T, rochedb::Error>` |

Still pending:

- transaction API;
- update / patch / delete / list / count APIs;
- dump / import / backup / restore APIs;
- metrics / universe sync / recovery APIs;
- native TCP driver with timeout/retry/pooling.

## Development

```bash
cargo fmt
ROCHEDB_CORE_DIR=/path/to/rochedb cargo test
```

This package intentionally starts as a thin C ABI wrapper. A native TCP driver
can be added later without changing the safe embedded API.
