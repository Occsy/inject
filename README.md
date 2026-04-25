# inject

A simple, file-based key-value database library for Rust.

Data is stored in plain-text `.dat` files in the format `key: value`.  
All write operations go through a `.tmp` file first and are then atomically renamed to `.dat`, ensuring the database is never left in a corrupt state.

---

## Features

- Plain-text `.dat` file storage — human readable and easy to inspect
- Atomic writes via `.tmp` → `.dat` rename
- Built-in logger that tracks every read, write, update, and delete operation
- Simple API through a single `Instance` struct

---

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
inject = { version = "0.1.0" }
```

---

## Quick Start

```rust
use inject::shrub::Instance;

fn main() {
    let mut db = Instance::default(); // uses ./default.dat

    // Write a new key-value pair
    db.set_key_value("name".to_string(), "Alice".to_string());
    db.write_pair().unwrap();

    // Update an existing key
    db.set_key_value("name".to_string(), "Bob".to_string());
    db.update_pair().unwrap();

    // Delete a key
    db.set_key("name".to_string());
    db.delete_pair().unwrap();
}
```

---

## API

### `Instance`

The main entry point for all database operations.

| Method | Description |
|--------|-------------|
| `Instance::default()` | Creates an instance backed by `./default.dat` |
| `set_key_value(key, value)` | Sets the active key and value |
| `set_key(key)` | Sets only the active key |
| `set_value(value)` | Sets only the active value |
| `write_pair()` | Writes the active key-value pair to disk (skips if key exists) |
| `update_pair()` | Updates an existing key, or inserts it if it does not exist |
| `delete_pair()` | Deletes the entry matching the active key |
| `read_data()` | Reads the file from disk into memory |
| `get_key()` | Returns a copy of the active key |
| `get_value()` | Returns a copy of the active value |
| `get_file()` | Returns a snapshot of the underlying `KnownFile` |
| `set_log(bool)` | Enables or disables operation logging |
| `get_log()` | Returns whether logging is enabled |
| `get_logger()` | Returns a snapshot of the internal `Logger` |
| `set_logger(logger)` | Replaces the internal logger |

---

### `Logger`

Tracks every operation performed during a session, separated by type.

| Method | Description |
|--------|-------------|
| `get_read()` | Returns all pairs that were read |
| `get_added()` | Returns all pairs that were added |
| `get_deleted()` | Returns all pairs that were deleted |
| `get_updated()` | Returns all pairs that were updated |
| `run_logger(action, pair)` | Routes a pair into the correct log bucket |

---

### `KnownFile`

Low-level file handle. You normally interact with this through `Instance`, but it is public for advanced use.

| Method | Description |
|--------|-------------|
| `get_path()` | Returns the file path |
| `set_path(path)` | Updates the stored path (does not move the file on disk) |
| `get_contents()` | Returns a clone of the in-memory content vector |
| `set_contents(vec)` | Replaces the in-memory content vector |
| `append_contents(key, value)` | Adds a pair to the in-memory content |
| `remove_contents(key)` | Removes all entries matching the key |
| `update_by_key(key, value)` | Updates an existing key's value in-place |
| `truncate_contents()` | Clears the in-memory content |
| `blank()` | Deletes the file from disk |

---

## Error Handling

All fallible methods return `Result<(), TErrors>`. The full list of error variants:

| Variant | Meaning |
|---------|---------|
| `ContentsEmpty` | The file exists but has no data |
| `ReadBytesError` | Failed to read from the file |
| `WriteBytesError` | Failed to write to the file |
| `FileCloneError` | Failed to clone a file handle |
| `IndexError` | A key lookup returned no result |
| `FileIOError` | General I/O error |
| `FileCreateError` | The file could not be created |
| `FlushError` | Failed to flush the write buffer |
| `DirError` | Failed to read the working directory |
| `TempCreate` | Failed to create the `.tmp` file |
| `TempReplace` | Failed to rename `.tmp` to `.dat` |
| `RenameError` | A file rename operation failed |

---

## Examples

### Writing multiple pairs

```rust
use inject::shrub::Instance;

let mut db = Instance::default();

let pairs = vec![
    ("username", "alice"),
    ("language", "Rust"),
    ("version",  "1.0"),
];

for (key, value) in pairs {
    db.set_key_value(key.to_string(), value.to_string());
    db.write_pair().unwrap();
}
```

### Reading the database back from disk

```rust
use inject::shrub::Instance;

let mut db = Instance::default();
db.read_data().unwrap();

for (key, value) in db.get_file().get_contents() {
    println!("{key} = {value}");
}
```

### Using the logger

```rust
use inject::shrub::Instance;

let mut db = Instance::default();

db.set_key_value("score".to_string(), "42".to_string());
db.write_pair().unwrap();

db.set_key_value("score".to_string(), "99".to_string());
db.update_pair().unwrap();

let logger = db.get_logger();
println!("Added:   {:?}", logger.get_added());
println!("Updated: {:?}", logger.get_updated());
```

### Disabling the logger

```rust
use inject::shrub::Instance;

let mut db = Instance::default();
db.set_log(false);

db.set_key_value("key".to_string(), "value".to_string());
db.write_pair().unwrap();
// Nothing is recorded in the logger
```

### Error handling

```rust
use inject::shrub::{Instance, TErrors};

let mut db = Instance::default();
db.set_key_value("city".to_string(), "Madrid".to_string());

match db.write_pair() {
    Ok(()) => println!("Written successfully"),
    Err(TErrors::FileCreateError) => eprintln!("Could not create the file"),
    Err(e) => eprintln!("Unexpected error: {e:?}"),
}
```

---

## File Format

The `.dat` files are plain text, one entry per line:

```
username: alice
language: Rust
version: 1.0
```

This makes them easy to inspect, edit manually, or migrate to other formats.

---

## License

MIT
