# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-04-24

### Added

- `shrub::Instance` — main interface for all key-value database operations
  - `write_pair()` — writes the active key-value pair to disk, skipping duplicates
  - `update_pair()` — updates an existing key's value, or inserts it if it does not exist
  - `delete_pair()` — deletes the entry matching the active key
  - `read_data()` — reads the `.dat` file from disk into memory
  - `set_key_value()`, `set_key()`, `set_value()` — setters for the active key and value
  - `get_key()`, `get_value()`, `get_file()` — getters for current state
  - `set_log()` / `get_log()` — enable or disable operation logging
  - `set_logger()` / `get_logger()` — replace or retrieve the internal logger
  - `kv_to_contents()` — appends the active key-value pair to the in-memory content

- `shrub::Logger` — session logger that tracks operations by type
  - Separate buckets for `read`, `added`, `deleted`, and `updated` operations
  - `run_logger()` — routes a pair into the correct bucket via `LoggerActions`
  - Individual `add_*` and `get_*` methods for each bucket

- `shrub::LoggerActions` — enum to select which log bucket to write to (`Read`, `Added`, `Deleted`, `Updated`)

- `shrub::TErrors` — error enum covering all possible failure modes (`ContentsEmpty`, `ReadBytesError`, `WriteBytesError`, `FileCloneError`, `IndexError`, `FileIOError`, `FileCreateError`, `FlushError`, `DirError`, `TempCreate`, `TempReplace`, `RenameError`)

- `file_manip::KnownFile` — low-level file handle storing a path and in-memory content vector
  - `init()` — creates the `.dat` file on disk if it does not exist
  - `create_file()` — persists in-memory content to disk via atomic `.tmp` → `.dat` rename
  - `read_file()` — reads and parses the `.dat` file from disk by path
  - `append_contents()`, `remove_contents()`, `update_by_key()`, `set_contents()`, `truncate_contents()` — in-memory content manipulation
  - `blank()` — deletes the file from disk
  - `get_path()`, `set_path()`, `get_contents()` — accessors

### Notes

- All write operations use a `.tmp` intermediate file and an atomic rename to prevent data corruption on crash
- Key comparisons are whitespace-insensitive throughout (`trim()` applied before every comparison)
- The internal `File` descriptor was removed from `KnownFile`; reads always open the file fresh by path to avoid stale handles after renames
- Full documentation comments and runnable examples on every public item