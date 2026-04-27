//! # injekt
//!
//! A simple file-based key-value database library.
//!
//! Data is stored in `.dat` files in a compact binary format: each entry is written as
//! a 4-byte little-endian key length, the key bytes, a 4-byte little-endian value length,
//! and the value bytes. All write operations go through a `.tmp` file first and are then
//! atomically renamed to `.dat`, ensuring the database is never left in a corrupt state.
//!
//! ## Quick start
//!
//! ```no_run
//! use injekt::shrub::Instance;
//!
//! let mut db = Instance::default();
//!
//! db.set_key_value("name".to_string(), "Alice".to_string());
//! db.write_pair().unwrap();
//!
//! db.set_key_value("name".to_string(), "Bob".to_string());
//! db.update_pair().unwrap();
//!
//! db.set_key("name".to_string());
//! db.delete_pair().unwrap();
//! ```
pub mod shrub {
    use crate::file_manip::KnownFile;
    use std::path::Path;

    /// All possible errors that can occur during database operations.
    ///
    /// Every public method that interacts with the filesystem returns
    /// `Result<(), TErrors>` so callers can handle failures explicitly.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use injekt::shrub::{Instance, TErrors};
    ///
    /// let mut db = Instance::default();
    /// db.set_key_value("city".to_string(), "Madrid".to_string());
    ///
    /// match db.write_pair() {
    ///     Ok(()) => println!("Written successfully"),
    ///     Err(TErrors::FileCreateError) => eprintln!("Could not create the file"),
    ///     Err(e) => eprintln!("Unexpected error: {e:?}"),
    /// }
    /// ```
    #[derive(Debug, PartialEq)]
    pub enum TErrors {
        /// The file exists but contains no data.
        ContentsEmpty,
        /// Failed to read bytes from the file.
        ReadBytesError,
        /// Failed to write bytes to the file.
        WriteBytesError,
        /// Failed to clone a file handle.
        FileCloneError,
        /// A key index lookup returned no result.
        IndexError,
        /// A general file I/O error occurred.
        FileIOError,
        /// The file could not be created.
        FileCreateError,
        /// Failed to flush the file buffer.
        FlushError,
        /// Failed to read the working directory.
        DirError,
        /// Failed to create the temporary `.tmp` file.
        TempCreate,
        /// Failed to replace the `.tmp` file with the `.dat` file.
        TempReplace,
        /// Failed to rename a file.
        RenameError,
    }

    /// Determines which log bucket a recorded operation is placed into.
    ///
    /// Pass one of these variants to [`Logger::run_logger`] to route a
    /// `(key, value)` pair into the correct log list.
    ///
    /// # Example
    ///
    /// ```
    /// use injekt::shrub::{Logger, LoggerActions};
    ///
    /// let mut logger = Logger::default();
    /// logger.run_logger(
    ///     LoggerActions::Added,
    ///     ("username".to_string(), "alice".to_string()),
    /// );
    /// assert_eq!(logger.get_added().len(), 1);
    /// ```
    pub enum LoggerActions {
        /// Log a read operation.
        Read,
        /// Log an add operation.
        Added,
        /// Log a delete operation.
        Deleted,
        /// Log an update operation.
        Updated,
    }

    /// Tracks every key-value operation performed during a session.
    ///
    /// Each category of operation (read, added, deleted, updated) is stored in
    /// its own `Vec<(String, String)>`. The logger is embedded inside [`Instance`]
    /// and is updated automatically when `log` is set to `true`.
    ///
    /// # Example
    ///
    /// ```
    /// use injekt::shrub::Logger;
    ///
    /// let mut logger = Logger::default();
    /// logger.add_add(("key".to_string(), "value".to_string()));
    /// assert_eq!(logger.get_added(), vec![("key".to_string(), "value".to_string())]);
    /// ```
    pub struct Logger {
        read: Vec<(String, String)>,
        added: Vec<(String, String)>,
        deleted: Vec<(String, String)>,
        updated: Vec<(String, String)>,
    }

    impl Default for Logger {
        /// Creates a new [`Logger`] with all buckets empty.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let logger = Logger::default();
        /// assert!(logger.get_read().is_empty());
        /// assert!(logger.get_added().is_empty());
        /// assert!(logger.get_deleted().is_empty());
        /// assert!(logger.get_updated().is_empty());
        /// ```
        fn default() -> Self {
            Self {
                read: Vec::new(),
                added: Vec::new(),
                deleted: Vec::new(),
                updated: Vec::new(),
            }
        }
    }

    impl Logger {
        /// Returns a clone of all key-value pairs that were read during this session.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_read(("name".to_string(), "Alice".to_string()));
        /// assert_eq!(logger.get_read(), vec![("name".to_string(), "Alice".to_string())]);
        /// ```
        pub fn get_read(&self) -> Vec<(String, String)> {
            self.read.clone()
        }

        /// Returns a clone of all key-value pairs that were added during this session.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_add(("age".to_string(), "30".to_string()));
        /// assert_eq!(logger.get_added(), vec![("age".to_string(), "30".to_string())]);
        /// ```
        pub fn get_added(&self) -> Vec<(String, String)> {
            self.added.clone()
        }

        /// Returns a clone of all key-value pairs that were deleted during this session.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_deleted(("city".to_string(), "Madrid".to_string()));
        /// assert_eq!(logger.get_deleted(), vec![("city".to_string(), "Madrid".to_string())]);
        /// ```
        pub fn get_deleted(&self) -> Vec<(String, String)> {
            self.deleted.clone()
        }

        /// Returns a clone of all key-value pairs that were updated during this session.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_updated(("score".to_string(), "99".to_string()));
        /// assert_eq!(logger.get_updated(), vec![("score".to_string(), "99".to_string())]);
        /// ```
        pub fn get_updated(&self) -> Vec<(String, String)> {
            self.updated.clone()
        }

        /// Records a key-value pair as having been read.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_read(("name".to_string(), "Alice".to_string()));
        /// assert_eq!(logger.get_read().len(), 1);
        /// ```
        pub fn add_read(&mut self, val: (String, String)) {
            self.read.push(val);
        }

        /// Records a key-value pair as having been added.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_add(("name".to_string(), "Alice".to_string()));
        /// assert_eq!(logger.get_added().len(), 1);
        /// ```
        pub fn add_add(&mut self, val: (String, String)) {
            self.added.push(val);
        }

        /// Records a key-value pair as having been deleted.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_deleted(("name".to_string(), "Alice".to_string()));
        /// assert_eq!(logger.get_deleted().len(), 1);
        /// ```
        pub fn add_deleted(&mut self, val: (String, String)) {
            self.deleted.push(val);
        }

        /// Records a key-value pair as having been updated.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::Logger;
        ///
        /// let mut logger = Logger::default();
        /// logger.add_updated(("name".to_string(), "Bob".to_string()));
        /// assert_eq!(logger.get_updated().len(), 1);
        /// ```
        pub fn add_updated(&mut self, val: (String, String)) {
            self.updated.push(val);
        }

        /// Routes a key-value pair into the correct log bucket based on `logger_action`.
        ///
        /// This is a convenience wrapper around the individual `add_*` methods.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::shrub::{Logger, LoggerActions};
        ///
        /// let mut logger = Logger::default();
        /// logger.run_logger(LoggerActions::Deleted, ("name".to_string(), "Alice".to_string()));
        /// assert_eq!(logger.get_deleted().len(), 1);
        /// ```
        pub fn run_logger(&mut self, logger_action: LoggerActions, val: (String, String)) {
            match logger_action {
                LoggerActions::Read => self.add_read(val),
                LoggerActions::Added => self.add_add(val),
                LoggerActions::Deleted => self.add_deleted(val),
                LoggerActions::Updated => self.add_updated(val),
            }
        }
    }

    /// The main interface for interacting with the key-value database.
    ///
    /// `Instance` wraps a [`KnownFile`] and exposes high-level operations such as
    /// writing, reading, updating, and deleting key-value pairs. It also embeds a
    /// [`Logger`] that tracks every operation when `log` is enabled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use injekt::shrub::Instance;
    ///
    /// let mut db = Instance::default();
    /// db.set_key_value("language".to_string(), "Rust".to_string());
    /// db.write_pair().unwrap();
    /// ```
    pub struct Instance {
        file: KnownFile,
        key: String,
        value: String,
        log: bool,
        logger: Logger,
    }

    impl Default for Instance {
        /// Creates a new [`Instance`] backed by `./default.dat`.
        ///
        /// Logging is enabled by default. The file is created on disk if it does
        /// not already exist.
        ///
        /// # Panics
        ///
        /// Panics if `./default.dat` cannot be created or opened.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// ```
        fn default() -> Self {
            Self {
                file: KnownFile::init("./default.dat".to_string())
                    .expect("Unable to initiate KnownFile"),
                key: "".to_string(),
                value: "".to_string(),
                log: true,
                logger: Logger::default(),
            }
        }
    }

    impl Instance {
        /// Appends the current `key` and `value` of the instance into the in-memory
        /// content of the underlying [`KnownFile`].
        ///
        /// This does **not** write to disk. Call [`Instance::write_pair`] to persist.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("country".to_string(), "Spain".to_string());
        /// db.kv_to_contents();
        /// assert_eq!(db.get_file().get_contents().len(), 1);
        /// ```
        pub fn kv_to_contents(&mut self) {
            let mut temp_file = self.get_file();

            let mut temp_contents: Vec<(String, String)> = temp_file.get_contents();

            temp_contents.push((self.get_key(), self.get_value()));

            temp_file.set_contents(temp_contents);

            self.set_file(temp_file);
        }

        /// Sets both the active `key` and `value` in a single call.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("animal".to_string(), "cat".to_string());
        /// assert_eq!(db.get_key(), "animal");
        /// assert_eq!(db.get_value(), "cat");
        /// ```
        pub fn set_key_value(&mut self, key: String, value: String) {
            self.key = key;
            self.value = value;
        }

        /// Sets the active key used by the next database operation.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key("color".to_string());
        /// assert_eq!(db.get_key(), "color");
        /// ```
        pub fn set_key(&mut self, key: String) {
            self.key = key;
        }

        /// Sets the active value used by the next database operation.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_value("blue".to_string());
        /// assert_eq!(db.get_value(), "blue");
        /// ```
        pub fn set_value(&mut self, value: String) {
            self.value = value;
        }

        /// Sets the file path directly on the underlying [`KnownFile`].
        ///
        /// This is a lower-level alternative to [`Instance::set_file`] when only the path
        /// needs to change. Note: changing the path does **not** move the file on disk.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_file_path("./other.dat".to_string());
        /// assert_eq!(db.get_file_path(), "./other.dat");
        /// ```
        pub fn set_file_path(&mut self, path: String) {
            self.file.path = path;
        }

        /// Replaces the in-memory content of the underlying [`KnownFile`] directly.
        ///
        /// This does **not** write to disk. Use [`Instance::write_pair`] or
        /// [`Instance::update_pair`] to persist changes.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_file_content(vec![("key".to_string(), "value".to_string())]);
        /// assert_eq!(db.get_file_content().len(), 1);
        /// ```
        pub fn set_file_content(&mut self, content: Vec<(String, String)>) {
            self.file.content = content;
        }

        /// Returns a copy of the file path stored in the underlying [`KnownFile`].
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// assert_eq!(db.get_file_path(), "./default.dat");
        /// ```
        pub fn get_file_path(&self) -> String {
            self.file.path.to_string()
        }

        /// Returns a clone of the in-memory content vector from the underlying [`KnownFile`].
        ///
        /// Each element is a `(key, value)` tuple. The vector reflects the last state loaded
        /// from disk (via [`Instance::read_data`]) plus any pending in-memory changes.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_file_content(vec![("lang".to_string(), "Rust".to_string())]);
        /// assert_eq!(db.get_file_content()[0].0, "lang");
        /// ```
        pub fn get_file_content(&self) -> Vec<(String, String)> {
            self.file.content.clone()
        }

        /// Replaces the underlying [`KnownFile`] with a new one.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut db = Instance::default();
        /// let new_file = KnownFile { path: "./other.dat".to_string(), content: Vec::new() };
        /// db.set_file(new_file);
        /// ```
        pub fn set_file(&mut self, file: KnownFile) {
            self.file = file;
        }

        /// Enables or disables automatic logging of database operations.
        ///
        /// When `log` is `true`, every `write_pair`, `delete_pair`, and
        /// `update_pair` call records the operation in the internal [`Logger`].
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_log(false);
        /// assert_eq!(db.get_log(), false);
        /// ```
        pub fn set_log(&mut self, log: bool) {
            self.log = log;
        }

        /// Returns whether logging is currently enabled.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// assert_eq!(db.get_log(), true); // logging is on by default
        /// ```
        pub fn get_log(&self) -> bool {
            self.log
        }

        /// Replaces the internal [`Logger`] with the provided one.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::{Instance, Logger};
        ///
        /// let mut db = Instance::default();
        /// let fresh_logger = Logger::default();
        /// db.set_logger(fresh_logger);
        /// ```
        pub fn set_logger(&mut self, logger: Logger) {
            self.logger = logger;
        }

        /// Returns a snapshot of the internal [`Logger`].
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("x".to_string(), "1".to_string());
        /// db.write_pair().unwrap();
        ///
        /// let logger = db.get_logger();
        /// assert_eq!(logger.get_added().len(), 1);
        /// ```
        pub fn get_logger(&self) -> Logger {
            Logger {
                read: self.logger.get_read(),
                added: self.logger.get_added(),
                deleted: self.logger.get_deleted(),
                updated: self.logger.get_updated(),
            }
        }

        /// Returns a copy of the active key.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key("planet".to_string());
        /// assert_eq!(db.get_key(), "planet");
        /// ```
        pub fn get_key(&self) -> String {
            String::from(&self.key)
        }

        /// Returns a copy of the active value.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_value("Earth".to_string());
        /// assert_eq!(db.get_value(), "Earth");
        /// ```
        pub fn get_value(&self) -> String {
            String::from(&self.value)
        }

        /// Returns a snapshot of the underlying [`KnownFile`].
        ///
        /// The snapshot shares the same `path` and `content` but is an independent
        /// copy, so mutations to it do not affect the instance until [`Instance::set_file`]
        /// is called.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// let file = db.get_file();
        /// assert_eq!(file.get_path(), "./default.dat");
        /// ```
        pub fn get_file(&self) -> KnownFile {
            KnownFile {
                path: self.file.get_path(),
                content: self.file.get_contents(),
            }
        }

        /// Writes the active key-value pair to the database file.
        ///
        /// Before writing, the file is read from disk so that existing pairs are
        /// not overwritten. If the key already exists, the write is skipped to
        /// avoid duplicates. Use [`Instance::update_pair`] to change an existing value.
        ///
        /// If logging is enabled the pair is recorded in the `added` log bucket.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors`] if reading or writing the file fails.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("fruit".to_string(), "apple".to_string());
        /// db.write_pair().unwrap();
        ///
        /// // Writing the same key again is a no-op
        /// db.set_value("banana".to_string());
        /// db.write_pair().unwrap();
        ///
        /// let contents = db.get_file().get_contents();
        /// assert_eq!(contents[0].1, "apple");
        /// ```
        pub fn write_pair(&mut self) -> Result<(), TErrors> {
            self.read_data()?;

            if self.log {
                self.logger.add_add((self.get_key(), self.get_value()));
            }

            if !self.search_vec() {
                self.file.append_contents(self.get_key(), self.get_value());
                self.file.create_file()?;
            }

            Ok(())
        }

        /// Reads the database file from disk and updates the in-memory content.
        ///
        /// If the file does not exist it is created as an empty file. If the file
        /// exists but is empty, a message is printed and `Ok(())` is returned.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::ReadBytesError`] if the file exists but cannot be read.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.read_data().unwrap();
        /// // db.get_file().get_contents() now reflects what is on disk
        /// ```
        pub fn read_data(&mut self) -> Result<(), TErrors> {
            let file_path = self.file.get_path();

            let wrapped_path: &Path = &Path::new(&file_path);

            if !wrapped_path.exists() {
                self.file.create_file()?;
                return Ok(());
            }

            let Err(err) = self.file.read_file() else {
                return Ok(());
            };

            if err == TErrors::ContentsEmpty {
                println!("contents empty");
                return Ok(());
            } else {
                return Err(TErrors::ReadBytesError);
            }
        }

        /// Returns `true` if the active key already exists in the in-memory content.
        ///
        /// Comparison is done after trimming whitespace from both sides so that
        /// keys parsed from disk (which may have surrounding spaces) are matched
        /// correctly.
        fn search_vec(&self) -> bool {
            let mut exists: bool = false;
            for (x, _y) in self.file.get_contents().iter() {
                exists = Self::trim_str(x.clone()) == Self::trim_str(self.get_key());
                if exists == true {
                    break;
                }
            }
            exists
        }

        /// Trims leading and trailing whitespace from a `String`.
        fn trim_str(str: String) -> String {
            str.trim().to_string()
        }

        /// Deletes the key-value pair identified by the active key from the database.
        ///
        /// The file is first read from disk to ensure the in-memory state is
        /// current. If logging is enabled the deleted pair (or a descriptive
        /// message if the key was not found) is recorded in the `deleted` log bucket.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors`] if reading or writing the file fails.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        ///
        /// db.set_key_value("temp".to_string(), "42".to_string());
        /// db.write_pair().unwrap();
        ///
        /// db.set_key("temp".to_string());
        /// db.delete_pair().unwrap();
        ///
        /// db.read_data().unwrap();
        /// assert!(db.get_file().get_contents().is_empty());
        /// ```
        pub fn delete_pair(&mut self) -> Result<(), TErrors> {
            self.read_data()?;

            if self.get_log() {
                let inner_contents = self.get_file().get_contents();
                self.logger.add_deleted((
                    self.get_key(),
                    if inner_contents.is_empty() {
                        "DB is empty".to_string()
                    } else {
                        let found_key = inner_contents
                            .into_iter()
                            .filter(|(x, _y)| {
                                Self::trim_str(String::from(x)) == Self::trim_str(self.get_key())
                            })
                            .collect::<Vec<(String, String)>>();
                        if found_key.is_empty() {
                            format!("DB read. Does not contain: {}", self.get_key())
                        } else {
                            String::from(&found_key[0].1)
                        }
                    },
                ));
            }

            self.file.remove_contents(self.get_key());

            self.file.create_file()?;

            Ok(())
        }

        /// Updates the value for the active key, or inserts the pair if the key
        /// does not yet exist.
        ///
        /// The file is read from disk before any modification to keep the in-memory
        /// state current. If logging is enabled the new pair is recorded in the
        /// `updated` log bucket regardless of whether it was an insert or an update.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors`] if reading or writing the file fails, or if the key
        /// index cannot be found during the update.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        ///
        /// // Insert a new pair
        /// db.set_key_value("score".to_string(), "10".to_string());
        /// db.write_pair().unwrap();
        ///
        /// // Update the existing pair
        /// db.set_key_value("score".to_string(), "99".to_string());
        /// db.update_pair().unwrap();
        ///
        /// db.read_data().unwrap();
        /// let contents = db.get_file().get_contents();
        /// assert_eq!(contents[0].1, "99");
        /// ```
        pub fn update_pair(&mut self) -> Result<(), TErrors> {
            self.read_data()?;

            if self.search_vec() {
                self.file.update_by_key(self.get_key(), self.get_value())?;
            } else {
                self.file.append_contents(self.get_key(), self.get_value());
            }

            if self.get_log() {
                self.logger.add_updated((self.get_key(), self.get_value()));
            }

            self.file.create_file()
        }
    }
}

#[allow(dead_code)]
pub mod file_manip {
    use std::ffi::OsStr;
    use std::fs::{File, read_dir};
    use std::io::BufWriter;
    use std::io::{Read, Write};
    use std::path::Path;

    use crate::shrub::TErrors;

    /// Low-level file handle that stores a path, and the parsed in-memory content
    /// of a `.dat` database file.
    ///
    /// All persistence operations (create, read, update, delete) are methods on
    /// this struct. Higher-level logic lives in [`crate::shrub::Instance`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use injekt::shrub::Instance;
    ///
    /// let mut db = Instance::default();
    /// db.set_key_value("hello".to_string(), "world".to_string());
    /// db.write_pair().unwrap();
    /// ```
    pub struct KnownFile {
        pub path: String,
        pub content: Vec<(String, String)>,
    }

    impl KnownFile {
        /// Creates a new [`KnownFile`] for the given path.
        ///
        /// If the file does not exist it is created as an empty file. If it already
        /// exists it is left untouched. The in-memory `content` always starts empty;
        /// call [`KnownFile::read_file`] to populate it from disk.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::FileCreateError`] if the file cannot be created.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// assert_eq!(db.get_file().get_path(), "./default.dat");
        /// assert!(db.get_file().get_contents().is_empty());
        /// ```
        pub(crate) fn init(path: String) -> Result<Self, TErrors> {
            // Create the file only if it does not already exist; otherwise leave it untouched.
            if !std::path::Path::new(&path).exists() {
                File::create_new(&path).map_err(|_| TErrors::FileCreateError)?;
            }
            Ok(Self {
                path,
                content: Vec::new(),
            })
        }

        /// Deletes the file at `self.path` from disk.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::FileIOError`] if the file cannot be removed.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let db = Instance::default();
        /// db.get_file().blank().unwrap(); // file is now removed from disk
        /// ```
        pub fn blank(&self) -> Result<(), TErrors> {
            std::fs::remove_file(String::from(&self.path)).map_err(|_| TErrors::FileIOError)
        }

        /// Clears the in-memory content vector without touching the file on disk.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.append_contents("a".to_string(), "1".to_string());
        /// kf.truncate_contents();
        /// assert!(kf.get_contents().is_empty());
        /// ```
        pub fn truncate_contents(&mut self) {
            self.set_contents(Vec::new());
        }

        /// Writes the current in-memory content to a `.tmp` file alongside the `.dat`.
        ///
        /// Each entry is serialised in a binary length-prefixed format:
        /// - 4 bytes (u32, little-endian): byte length of the key
        /// - N bytes: key encoded as UTF-8
        /// - 4 bytes (u32, little-endian): byte length of the value
        /// - M bytes: value encoded as UTF-8
        ///
        /// The `.tmp` file is created fresh (or truncated if it already exists) so that
        /// a crash mid-write leaves the original `.dat` intact.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors`] if the temp file cannot be created, written, or flushed.
        fn run_temp(&self) -> Result<(), TErrors> {
            let mut wrapped_path: &Path = &Path::new(&self.path);

            let new_ext = wrapped_path.with_extension("tmp");

            wrapped_path = new_ext.as_path();

            let temp_file: File = File::create_new(wrapped_path)
                .map_err(|_| {
                    return TErrors::FileCreateError;
                })
                .unwrap_or(
                    std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .open(wrapped_path)
                        .map_err(|_| {
                            return TErrors::FileIOError;
                        })?,
                );

            let mut writer = BufWriter::new(temp_file);

            for (key, val) in self.get_contents() {
                // Serialise each key-value pair in binary length-prefixed format.
                let key_bytes = key.as_bytes();
                let val_bytes = val.as_bytes();

                writer
                    .write_all(&(key_bytes.len() as u32).to_le_bytes())
                    .map_err(|_| TErrors::WriteBytesError)?;
                writer
                    .write_all(key_bytes)
                    .map_err(|_| TErrors::WriteBytesError)?;
                writer
                    .write_all(&(val_bytes.len() as u32).to_le_bytes())
                    .map_err(|_| TErrors::WriteBytesError)?;
                writer
                    .write_all(val_bytes)
                    .map_err(|_| TErrors::WriteBytesError)?;
            }

            writer.flush().map_err(|_| TErrors::FlushError)?;

            Ok(())
        }

        /// Renames every `.tmp` file in the current working directory to `.dat`.
        ///
        /// The rename is atomic on most operating systems, so the `.dat` file is
        /// never left in a partially-written state. Any existing `.dat` file with
        /// the same stem is replaced atomically.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors`] if the directory cannot be read or a rename fails.
        fn replace_temp(&self) -> Result<(), TErrors> {
            for files in read_dir("./").map_err(|_| TErrors::DirError)? {
                let file = files.map_err(|_| TErrors::DirError)?;
                if file.path().is_file() {
                    if file
                        .path()
                        .extension()
                        .unwrap_or(OsStr::new(""))
                        .to_str()
                        .expect("unable to convert file path to string.")
                        .contains("tmp")
                    {
                        let new_ext = file.path().with_extension("dat");

                        std::fs::rename(
                            file.path().to_str().expect("Unable to convert to str"),
                            new_ext
                                .as_path()
                                .to_str()
                                .expect("Unable to convert to str"),
                        )
                        .map_err(|_| TErrors::RenameError)?;
                    }
                }
            }

            Ok(())
        }

        /// Persists the in-memory content to disk.
        ///
        /// Internally this calls [`KnownFile::run_temp`] to write a `.tmp` file and
        /// then [`KnownFile::replace_temp`] to atomically rename it to `.dat`.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::TempCreate`] if writing the temp file fails, or
        /// [`TErrors::TempReplace`] if the rename step fails.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("key".to_string(), "value".to_string());
        /// db.write_pair().unwrap(); // content is now on disk
        /// ```
        pub(crate) fn create_file(&self) -> Result<(), TErrors> {
            self.run_temp().map_err(|err| {
                println!("Error: {err:?}");
                return TErrors::TempCreate;
            })?;
            self.replace_temp().map_err(|err| {
                println!("Error: {err:?}");
                return TErrors::TempReplace;
            })?;
            Ok(())
        }

        /// Reads the `.dat` file from disk and updates the in-memory content.
        ///
        /// The file is opened fresh by path on every call, so the result always
        /// reflects the current state on disk regardless of previous writes.
        /// The file is parsed using the binary length-prefixed format written by
        /// [`KnownFile::run_temp`]: each record is a u32 key-length, the key bytes,
        /// a u32 value-length, and the value bytes (all little-endian). Truncated or
        /// malformed records at the end of the buffer are silently ignored.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::ReadBytesError`] if the file cannot be opened or read.
        /// Returns [`TErrors::ContentsEmpty`] if the file is empty.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use injekt::shrub::Instance;
        ///
        /// let mut db = Instance::default();
        /// db.set_key_value("lang".to_string(), "Rust".to_string());
        /// db.write_pair().unwrap();
        ///
        /// // A new instance reading the same file from disk
        /// let mut db2 = Instance::default();
        /// db2.read_data().unwrap();
        /// assert_eq!(db2.get_file().get_contents()[0].0, "lang");
        /// ```
        pub(crate) fn read_file(&mut self) -> Result<(), TErrors> {
            // Open the .dat file fresh by path to avoid stale handles after atomic renames.
            let mut fresh_file = File::open(&self.path).map_err(|_| TErrors::ReadBytesError)?;

            let mut buffer: Vec<u8> = Vec::new();
            fresh_file
                .read_to_end(&mut buffer)
                .map_err(|_| TErrors::ReadBytesError)?;

            if buffer.is_empty() {
                return Err(TErrors::ContentsEmpty);
            }

            // Deserialise the binary length-prefixed format.
            let mut parsed_content: Vec<(String, String)> = Vec::new();
            let mut cursor: usize = 0;

            while cursor + 4 <= buffer.len() {
                // Read the key length prefix.
                let key_len = u32::from_le_bytes(
                    buffer[cursor..cursor + 4]
                        .try_into()
                        .map_err(|_| TErrors::ReadBytesError)?,
                ) as usize;
                cursor += 4;

                if cursor + key_len > buffer.len() {
                    break;
                }
                let key = String::from_utf8(buffer[cursor..cursor + key_len].to_vec())
                    .map_err(|_| TErrors::ReadBytesError)?;
                cursor += key_len;

                if cursor + 4 > buffer.len() {
                    break;
                }
                // Read the value length prefix.
                let val_len = u32::from_le_bytes(
                    buffer[cursor..cursor + 4]
                        .try_into()
                        .map_err(|_| TErrors::ReadBytesError)?,
                ) as usize;
                cursor += 4;

                if cursor + val_len > buffer.len() {
                    break;
                }
                let value = String::from_utf8(buffer[cursor..cursor + val_len].to_vec())
                    .map_err(|_| TErrors::ReadBytesError)?;
                cursor += val_len;

                parsed_content.push((key, value));
            }

            self.truncate_contents();
            for (key, value) in parsed_content {
                self.append_contents(key, value);
            }

            Ok(())
        }

        /// Returns a copy of the file path.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let kf = KnownFile { path: "./mydb.dat".to_string(), content: Vec::new() };
        /// assert_eq!(kf.get_path(), "./mydb.dat");
        /// ```
        pub fn get_path(&self) -> String {
            String::from(&self.path)
        }

        /// Replaces the stored file path.
        ///
        /// Note: changing the path does **not** move the file on disk.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./old.dat".to_string(), content: Vec::new() };
        /// kf.set_path("./new.dat".to_string());
        /// assert_eq!(kf.get_path(), "./new.dat");
        /// ```
        pub fn set_path(&mut self, path: String) {
            self.path = path;
        }

        /// Returns a clone of the full in-memory content vector.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.append_contents("a".to_string(), "1".to_string());
        /// assert_eq!(kf.get_contents(), vec![("a".to_string(), "1".to_string())]);
        /// ```
        pub fn get_contents(&self) -> Vec<(String, String)> {
            self.content.clone()
        }

        /// Replaces the entire in-memory content vector.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.set_contents(vec![("x".to_string(), "10".to_string())]);
        /// assert_eq!(kf.get_contents().len(), 1);
        /// ```
        pub fn set_contents(&mut self, content: Vec<(String, String)>) {
            self.content = content;
        }

        /// Appends a single key-value pair to the in-memory content.
        ///
        /// This does **not** write to disk. Call [`KnownFile::create_file`] to persist.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.append_contents("version".to_string(), "1.0".to_string());
        /// assert_eq!(kf.get_contents().len(), 1);
        /// ```
        pub fn append_contents(&mut self, key: String, value: String) {
            let mut temp_vec = self.content.clone();

            temp_vec.push((key, value));

            self.set_contents(temp_vec);
        }

        /// Removes all entries whose key matches `key` (after trimming whitespace)
        /// from the in-memory content.
        ///
        /// This does **not** write to disk. Call [`KnownFile::create_file`] to persist.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.append_contents("keep".to_string(), "this".to_string());
        /// kf.append_contents("remove".to_string(), "me".to_string());
        /// kf.remove_contents("remove".to_string());
        /// assert_eq!(kf.get_contents().len(), 1);
        /// assert_eq!(kf.get_contents()[0].0, "keep");
        /// ```
        pub fn remove_contents(&mut self, key: String) {
            let trimmed_key = key.trim().to_string();
            let filtered_content: Vec<(String, String)> = self
                .get_contents()
                .into_iter()
                .filter(|(x, _y)| x.trim().to_string() != trimmed_key)
                .collect::<Vec<(String, String)>>();

            self.set_contents(filtered_content);
        }

        /// Updates the value of an existing key in-place, preserving its position
        /// in the content vector.
        ///
        /// The key lookup is done after trimming whitespace so keys read from disk
        /// are matched correctly. If the key does not exist,
        /// [`TErrors::IndexError`] is returned.
        ///
        /// This does **not** write to disk. Call [`KnownFile::create_file`] to persist.
        ///
        /// # Errors
        ///
        /// Returns [`TErrors::IndexError`] if the key is not found in the in-memory content.
        ///
        /// # Example
        ///
        /// ```
        /// use injekt::file_manip::KnownFile;
        ///
        /// let mut kf = KnownFile { path: "./data.dat".to_string(), content: Vec::new() };
        /// kf.append_contents("score".to_string(), "10".to_string());
        /// kf.update_by_key("score".to_string(), "99".to_string()).unwrap();
        /// assert_eq!(kf.get_contents()[0].1, "99");
        /// ```
        pub fn update_by_key(&mut self, key: String, new_value: String) -> Result<(), TErrors> {
            let trimmed_key = key.trim().to_string();

            let Some(key_index) = self
                .get_contents()
                .iter()
                .position(|x| x.0.trim() == trimmed_key)
            else {
                return Err(TErrors::IndexError);
            };

            let mut inner_content: Vec<(String, String)> = self.get_contents();

            inner_content[key_index] = (key, new_value);

            self.set_contents(inner_content);

            Ok(())
        }
    }
}
