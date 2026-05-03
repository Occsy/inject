# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---
## [1.3.5]
### Functionality 
- `Instance::write_file_contents` removed 
## [1.3.1]
### Fuctionality
- `Instance::write_all` added 
- README updated

## [1.3.0]
### Functionality
- `Instance::new` changed to `Instance::init` to match pattern seen in `KnownFile::init`
- Scope of `KnownFile.path` changed to pub(crate)
- Scope of `KnownFile.contents` changed to pub(crate)
- Scope of `KnownFile::blank` changed to pub(crate)
- updated README 

## [1.2.8]
### Documentation 
- made documentation about 'write_fil_contents'

## [1.2.7]

### Functionality 
- `write_file_contents` sets to experiemental 

## [1.2.6]

### Funcionality

- `write_file_contents` set to experimental

## [1.2.5]

### Revert to [1.2.3]

- `write_file_contents` reverted to older version due to bug where db replaced with current content instead of having it as an append. 

## [1.2.4]

### Performance
- `write_file_contents` removed write_pair from every iteration of key, value. flush applied once all values written to file. 

## [1.2.3]

### Error Handling
- Added `TErrors` to all error handles. 

## [1.2.2]

### Error Handling

- `TErrors`: implemented debug improving readable. 

## [1.2.0] - 2026-04-27

### Performance

- `Instance`: added `content_stale: bool` field — `read_data` now short-circuits immediately when the in-memory state is already current, eliminating all redundant disk reads within a session; a loop writing N pairs drops from N full file reads to 1
- `read_data`: sets `content_stale = false` on every success path (file created, file read, file empty); `file_blank` sets it back to `true` so the next operation re-reads correctly
- `run_temp`: replaced two-step `File::create_new` + `OpenOptions` fallback with a single `OpenOptions::new().write(true).create(true).truncate(true).open(...)` — one syscall instead of a potential two
- `read_file`: eliminated the intermediate `parsed_content` staging `Vec`; records are now pushed directly into `self.content` in a single pass
- `read_file`: buffer is now pre-sized via `metadata().len()` (falling back to 256) before `read_to_end`, avoiding repeated capacity-doubling reallocations
- `replace_temp`: replaced full `read_dir("./")` directory scan with a direct `fs::rename` from the known `.tmp` path to the known `.dat` path — O(directory entries) work reduced to a single syscall (applied prior to this release)
- `set_key_value`, `set_key`, `set_value`: trimming now only allocates when whitespace is actually present; clean inputs are moved directly into the field with zero allocation

### Changed

- Removed now-unused `std::ffi::OsStr` and `read_dir` imports left over from the old `replace_temp` directory scan
- Updated `read_data` doc comment to document the no-op behaviour when content is already current
- Updated `README.md` installation version from `1.1.0` to `1.2.0`

---

## [1.1.0] - 2026-04-27

### Breaking Changes

- All getter methods now return references instead of owned values; callers that need an owned copy must call `.to_owned()` / `.clone()` explicitly:
  - `Instance::get_key()`: `String` → `&str`
  - `Instance::get_value()`: `String` → `&str`
  - `Instance::get_file_path()`: `String` → `&str`
  - `Instance::get_file_content()`: `Vec<(String, String)>` → `&[(String, String)]`
  - `Instance::get_file()`: `KnownFile` → `&KnownFile`
  - `Instance::get_logger()`: `Logger` → `&Logger`
  - `KnownFile::get_path()`: `String` → `&str`
  - `KnownFile::get_contents()`: `Vec<(String, String)>` → `&[(String, String)]`
  - `Logger::get_read()`: `Vec<(String, String)>` → `&[(String, String)]`
  - `Logger::get_added()`: `Vec<(String, String)>` → `&[(String, String)]`
  - `Logger::get_deleted()`: `Vec<(String, String)>` → `&[(String, String)]`
  - `Logger::get_updated()`: `Vec<(String, String)>` → `&[(String, String)]`
- `KnownFile::remove_contents()`: parameter changed from `String` to `&str`

### Performance

- In-memory allocation count per operation reduced from **O(N)** to **O(1)** — costs no longer scale with the number of entries in the database
- `run_temp`: eliminated full content `Vec` clone on every call; now iterates `&self.content` directly — this affected every single write, update, and delete operation
- `search_vec`: eliminated two `String` allocations per iteration (via `trim_str`) and one full `Vec` clone; now uses a direct reference equality check with zero allocations
- `append_contents`: eliminated full `Vec` clone on every call; now calls `Vec::push` directly
- `remove_contents`: eliminated full `Vec` clone; now uses `Vec::retain` in-place
- `update_by_key`: eliminated two full `Vec` clones; now indexes `self.content` directly
- `kv_to_contents`: eliminated `KnownFile` clone and full content `Vec` clone; collapsed to a single `Vec::push`
- `delete_pair`: eliminated full `KnownFile` clone and full content `Vec` clone for the logging path; now uses `Iterator::find` with one `String` clone for the matched value only
- `blank`: eliminated a `String::from` clone; `remove_file` now receives `&self.path` directly
- `read_data`: eliminated a `get_path()` clone; path is now accessed as a field reference inline
- `write_file_contents`: replaced implicit per-item clones with one explicit `Vec::clone` before the loop; items are moved out of the owned `Vec` so no per-iteration clones are needed

### Changed

- `set_key_value`, `set_key`, and `set_value` now strip leading and trailing whitespace from inputs before storing — all keys and values are pre-normalised on entry, removing the need to trim at comparison time
- `search_vec`, `delete_pair` filter, `remove_contents`, and `update_by_key` updated to use direct equality now that keys are guaranteed pre-trimmed
- `truncate_contents`: replaced `set_contents(Vec::new())` with `Vec::clear()`
- `write_file_contents`: replaced `for_each` closure with a `for` loop using `?` for error propagation
- Removed `trim_str` private helper — no callers remained after normalization was moved to the setters
- Updated doc comments on all getter methods to reflect reference return types
- Updated `search_vec` doc comment: normalization-on-entry approach documented; trim mention removed
- Updated `remove_contents` doc comment: removed stale "after trimming whitespace" note
- Updated `update_by_key` doc comment: removed stale "after trimming whitespace" note
- Updated `README.md` installation version from `1.0.2` to `1.1.0` and revised API table descriptions to reflect reference-returning getters and the `&str` parameter on `remove_contents`

---

## [1.0.2] - 2026-04-27

### Added

- `shrub::Instance::file_blank()` — convenience wrapper around `KnownFile::blank()` that deletes the database file from disk directly through the instance; returns `TErrors::FileIOError` on failure
- `shrub::Instance::write_file_contents()` — persists the current in-memory content to disk by iterating over the content vector and calling `write_pair()` for each entry; if the active key is non-empty it is appended to the buffer before the loop so it is included in the write

### Changed

- Added `///` doc comments (with `# Errors` and `# Example` sections) to `file_blank` and `write_file_contents`, which were previously undocumented
- Added `file_blank` and `write_file_contents` to the `Instance` API table in `README.md`
- Updated installation version in `README.md` from `1.0.1` to `1.0.2`

---

## [1.0.1] - 2026-04-27

### Fixed

- Corrected all documentation that described `.dat` files as plain-text `key: value` — the actual storage format is binary length-prefixed records
- Fixed a typo in the `Logger::run_logger` doc comment (`"a enience wrapper"` → `"a convenience wrapper"`)
- Translated all Spanish inline comments in `file_manip` (`run_temp`, `read_file`, `KnownFile::init`) to English

### Changed

- Updated `README.md` installation version from `0.1.0` to `1.0.1`
- Updated `README.md` features section: replaced `"Plain-text .dat file storage — human readable and easy to inspect"` with `"Binary .dat file storage — compact and crash-safe"`
- Rewrote `README.md` File Format section to accurately describe the binary length-prefixed record layout (u32 key length + key bytes + u32 value length + value bytes, all little-endian)
- Updated `run_temp` and `read_file` doc comments to describe the binary format instead of the old `key: value\n` plain-text format
- Updated the crate-level `//!` doc to describe the binary storage format

### Added

- Added `///` doc comments (with `# Example` sections) to four previously undocumented public `Instance` methods: `set_file_path`, `set_file_content`, `get_file_path`, `get_file_content`
- Added missing `Instance` methods to the `README.md` API table: `kv_to_contents`, `set_file`, `get_file_path`, `set_file_path`, `get_file_content`, `set_file_content`
- Added missing `Logger` mutation methods to the `README.md` API table: `add_read`, `add_add`, `add_deleted`, `add_updated`

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
