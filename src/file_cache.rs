use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

pub struct FileCache {
    /// Set of import paths search for imports
    import_paths: Vec<PathBuf>,
    /// List file by import path
    cached_paths: HashMap<PathBuf, usize>,
    /// The actual file contents
    files: Vec<Rc<String>>,
}

/// When we resolve a file, we need to know its base compared to the import so
/// we can resolve the next import, and the full path on the filesystem.
/// Since the same filename can exists in multiple imports, we need to tell the
/// user exactly which file has errors/warnings.
#[derive(Clone, Debug)]
pub struct ResolvedFile {
    /// Full path on the filesystem
    pub full_path: PathBuf,
    /// Index into the file cache
    file_no: usize,
    /// Which import path was used, if any
    import_no: usize,
    // Base part relative to import
    base: PathBuf,
}

impl Default for FileCache {
    fn default() -> Self {
        FileCache::new()
    }
}

impl FileCache {
    /// Create a new file cache object
    pub fn new() -> Self {
        FileCache {
            import_paths: Vec::new(),
            cached_paths: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Add import path. This must be the canonicalized path
    pub fn add_import_path(&mut self, path: PathBuf) {
        self.import_paths.push(path);
    }

    /// Update the cache for the filename with the given contents
    pub fn set_file_contents(&mut self, path: &str, contents: String) {
        let pos = self.files.len();

        self.files.push(Rc::new(contents));

        self.cached_paths.insert(PathBuf::from(path), pos);
    }

    /// Get file with contents. This must be a file which was previously
    /// add to the cache
    pub fn get_file_contents(&mut self, file: &PathBuf) -> Rc<String> {
        let file_no = self.cached_paths[file];

        self.files[file_no].clone()
    }

    /// Populate the cache with absolute file path
    fn load_file(&mut self, path: &PathBuf) -> Result<usize, String> {
        if let Some(file_no) = self.cached_paths.get(path) {
            return Ok(*file_no);
        }

        // read the file
        let mut f = match File::open(&path) {
            Err(err_info) => {
                return Err(format!(
                    "cannot open file ‘{}’: {}",
                    path.display(),
                    err_info.to_string()
                ));
            }
            Ok(file) => file,
        };

        let mut contents = String::new();
        if let Err(e) = f.read_to_string(&mut contents) {
            return Err(format!(
                "failed to read file ‘{}’: {}",
                path.display(),
                e.to_string()
            ));
        }

        let pos = self.files.len();

        self.files.push(Rc::new(contents));

        self.cached_paths.insert(path.clone(), pos);

        Ok(pos)
    }

    /// Walk the import path to search for a file. If no import path is set up,
    /// return. Check each import path if the file can be found in a subdirectory
    /// of that path, and return the canonicalized path.
    pub fn resolve_file(
        &mut self,
        parent: Option<&ResolvedFile>,
        filename: &str,
    ) -> Result<ResolvedFile, String> {
        let path = PathBuf::from(filename);
        let mut start_import_no = 0;

        // first try relative to the parent
        if let Some(ResolvedFile {
            import_no, base, ..
        }) = parent
        {
            if self.import_paths.is_empty() {
                // we have no import paths, resolve by what's in the cache
                let full_path = base.join(path);
                let base = (&full_path.parent())
                    .expect("path should include filename")
                    .to_path_buf();

                let file_no = self.cached_paths[&full_path];

                return Ok(ResolvedFile {
                    full_path,
                    base,
                    import_no: 0,
                    file_no,
                });
            }

            let import_path = self.import_paths[*import_no].join(base);

            if let Ok(full_path) = import_path.join(path.clone()).canonicalize() {
                // strip the filename off and the import prefix for the base
                if let Ok(base) = &full_path
                    .parent()
                    .expect("path should include filename")
                    .strip_prefix(import_path)
                {
                    let file_no = self.load_file(&full_path)?;
                    let base = base.to_path_buf();

                    return Ok(ResolvedFile {
                        full_path,
                        base,
                        import_no: *import_no,
                        file_no,
                    });
                }
            }

            // start with the next import
            start_import_no = *import_no + 1;
        }

        if self.import_paths.is_empty() {
            // we have no import paths, resolve by what's in the cache
            let full_path = path;
            let base = (&full_path.parent())
                .expect("path should include filename")
                .to_path_buf();
            let file_no = self.cached_paths[&full_path];

            return Ok(ResolvedFile {
                full_path,
                base,
                import_no: 0,
                file_no,
            });
        }

        // walk over the import paths until we find one that resolves
        for i in 0..self.import_paths.len() {
            let import_no = (i + start_import_no) % self.import_paths.len();
            let import_path = &self.import_paths[import_no];

            // we want to prevent walking up the tree with .. or /
            if let Ok(full_path) = import_path.join(path.clone()).canonicalize() {
                // strip the filename off and the import prefix for the base
                if let Ok(base) = &full_path
                    .parent()
                    .expect("path should include filename")
                    .strip_prefix(import_path)
                {
                    let file_no = self.load_file(&full_path)?;
                    let base = base.to_path_buf();

                    return Ok(ResolvedFile {
                        full_path,
                        base,
                        import_no,
                        file_no,
                    });
                }
            }
        }

        Err(format!("file not found ‘{}’", filename))
    }
}
