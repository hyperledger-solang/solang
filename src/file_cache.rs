use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

pub struct FileCache {
    import_path: Vec<PathBuf>,
    // Find file by how they are identified in code
    by_name: HashMap<String, usize>,
    // Find file by operating system path
    by_path: HashMap<PathBuf, usize>,
    // The actual file cache
    files: Vec<Rc<String>>,
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
            import_path: Vec::new(),
            by_name: HashMap::new(),
            by_path: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Add import path. This must be the canonicalized path
    pub fn add_import_path(&mut self, path: PathBuf) {
        self.import_path.push(path);
    }

    /// Update the cache for the filename with the given contents
    pub fn set_file_contents(&mut self, filename: String, contents: String) {
        let pos = self.files.len();

        self.files.push(Rc::new(contents));

        self.by_name.insert(filename, pos);
    }

    /// Get file with contents. This must be a file which was previously
    /// add to the cache
    pub fn get_file_contents(&mut self, filename: &str) -> Rc<String> {
        let pos = self
            .by_name
            .get(filename)
            .expect("file should exist in cache already");

        self.files[*pos].clone()
    }

    /// Load the given file into the cache
    pub fn populate_cache(&mut self, filename: &str) -> Result<(), String> {
        if self.by_name.contains_key(filename) {
            // already in the cache
            return Ok(());
        }

        if let Some(path) = self.resolve_file(filename) {
            if let Some(pos) = self.by_path.get(&path) {
                // we found a different name for the same file
                self.by_name.insert(filename.to_string(), *pos);

                return Ok(());
            }

            // read the file
            let mut f = match File::open(&path) {
                Err(err_info) => {
                    return Err(format!(
                        "cannot open file ‘{}’: {}",
                        filename,
                        err_info.to_string()
                    ));
                }
                Ok(file) => file,
            };

            let mut contents = String::new();
            if let Err(e) = f.read_to_string(&mut contents) {
                return Err(format!(
                    "failed to read file ‘{}’: {}",
                    filename,
                    e.to_string()
                ));
            }

            let pos = self.files.len();

            self.files.push(Rc::new(contents));

            self.by_name.insert(filename.to_string(), pos);
            self.by_path.insert(path, pos);

            Ok(())
        } else {
            Err(format!("file not found ‘{}’", filename))
        }
    }

    /// Walk the import path to search for a file. If no import path is set up,
    /// return. Check each import path if the file can be found in a subdirectory
    /// of that path, and return the canonicalized path.
    fn resolve_file(&self, filename: &str) -> Option<PathBuf> {
        let path = PathBuf::from(filename);

        for i in &self.import_path {
            // we want to prevent walking up the tree with .. or /
            if let Ok(p) = i.join(path.clone()).canonicalize() {
                if p.starts_with(i) {
                    return Some(p);
                }
            }
        }

        None
    }
}
