use crate::parser::pt::Loc;
use crate::sema::ast;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::io::{prelude::*, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct FileResolver {
    /// Set of import paths search for imports
    import_paths: Vec<(Option<OsString>, PathBuf)>,
    /// List file by import path
    cached_paths: HashMap<PathBuf, usize>,
    /// The actual file contents
    files: Vec<Arc<str>>,
}

/// When we resolve a file, we need to know its base compared to the import so
/// we can resolve the next import, and the full path on the filesystem.
/// Since the same filename can exists in multiple imports, we need to tell the
/// user exactly which file has errors/warnings.
#[derive(Clone, Debug)]
pub struct ResolvedFile {
    /// Full path on the filesystem
    pub full_path: PathBuf,
    /// Which import path was used, if any
    import_no: usize,
    // Base part relative to import
    base: PathBuf,
}

impl Default for FileResolver {
    fn default() -> Self {
        FileResolver::new()
    }
}

impl FileResolver {
    /// Create a new file resolver
    pub fn new() -> Self {
        FileResolver {
            import_paths: Vec::new(),
            cached_paths: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Add import path
    pub fn add_import_path(&mut self, path: PathBuf) -> io::Result<()> {
        self.import_paths.push((None, path.canonicalize()?));
        Ok(())
    }

    /// Add import map
    pub fn add_import_map(&mut self, map: OsString, path: PathBuf) -> io::Result<()> {
        if self
            .import_paths
            .iter()
            .any(|(m, _)| m.as_ref() == Some(&map))
        {
            Err(Error::new(
                ErrorKind::Other,
                format!("duplicate mapping for ‘{}’", map.to_string_lossy()),
            ))
        } else {
            self.import_paths.push((Some(map), path.canonicalize()?));
            Ok(())
        }
    }

    /// Update the cache for the filename with the given contents
    pub fn set_file_contents(&mut self, path: &str, contents: String) {
        let pos = self.files.len();

        self.files.push(Arc::from(contents));

        self.cached_paths.insert(PathBuf::from(path), pos);
    }

    /// Get file with contents. This must be a file which was previously
    /// add to the cache
    pub fn get_file_contents_and_number(&self, file: &Path) -> (Arc<str>, usize) {
        let file_no = self.cached_paths[file];

        (self.files[file_no].clone(), file_no)
    }

    /// Populate the cache with absolute file path
    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        if self.cached_paths.get(path).is_some() {
            return Ok(());
        }

        // read the file
        let mut f = match File::open(&path) {
            Err(err_info) => {
                return Err(format!(
                    "cannot open file ‘{}’: {}",
                    path.display(),
                    err_info
                ));
            }
            Ok(file) => file,
        };

        let mut contents = String::new();
        if let Err(e) = f.read_to_string(&mut contents) {
            return Err(format!("failed to read file ‘{}’: {}", path.display(), e));
        }

        let pos = self.files.len();

        self.files.push(Arc::from(contents));

        self.cached_paths.insert(path.to_path_buf(), pos);

        Ok(())
    }

    /// Walk the import path to search for a file. If no import path is set up,
    /// return. Check each import path if the file can be found in a subdirectory
    /// of that path, and return the canonicalized path.
    pub fn resolve_file(
        &mut self,
        parent: Option<&ResolvedFile>,
        filename: &OsStr,
    ) -> Result<ResolvedFile, String> {
        let path = PathBuf::from(filename);

        // first check maps
        let mut iter = path.iter();
        if let Some(first_part) = iter.next() {
            let relpath: PathBuf = iter.collect();

            for (import_no, import) in self.import_paths.iter().enumerate() {
                if let (Some(mapping), import_path) = import {
                    if first_part == mapping {
                        // match!
                        if let Ok(full_path) = import_path.join(&relpath).canonicalize() {
                            self.load_file(&full_path)?;
                            let base = full_path
                                .parent()
                                .expect("path should include filename")
                                .to_path_buf();

                            return Ok(ResolvedFile {
                                full_path,
                                import_no,
                                base,
                            });
                        }
                    }
                }
            }
        }

        let mut start_import_no = 0;

        // first try relative to the parent
        if let Some(ResolvedFile {
            import_no, base, ..
        }) = parent
        {
            if self.import_paths.is_empty() {
                // we have no import paths, resolve by what's in the cache
                let full_path = base.join(&path);
                let base = full_path
                    .parent()
                    .expect("path should include filename")
                    .to_path_buf();

                return Ok(ResolvedFile {
                    full_path,
                    base,
                    import_no: 0,
                });
            } else if let Ok(full_path) = base.join(&path).canonicalize() {
                self.load_file(&full_path)?;
                let base = full_path
                    .parent()
                    .expect("path should include filename")
                    .to_path_buf();

                return Ok(ResolvedFile {
                    full_path,
                    base,
                    import_no: 0,
                });
            }

            // start with this import
            start_import_no = *import_no;
        }

        if self.import_paths.is_empty() {
            // we have no import paths, resolve by what's in the cache
            let full_path = path;
            let base = full_path
                .parent()
                .expect("path should include filename")
                .to_path_buf();

            return Ok(ResolvedFile {
                full_path,
                base,
                import_no: 0,
            });
        }

        // walk over the import paths until we find one that resolves
        for i in 0..self.import_paths.len() {
            let import_no = (i + start_import_no) % self.import_paths.len();

            if let (None, import_path) = &self.import_paths[import_no] {
                if let Ok(full_path) = import_path.join(&path).canonicalize() {
                    let base = full_path
                        .parent()
                        .expect("path should include filename")
                        .to_path_buf();
                    self.load_file(&full_path)?;

                    return Ok(ResolvedFile {
                        full_path,
                        import_no,
                        base,
                    });
                }
            }
        }

        Err(format!("file not found ‘{}’", filename.to_string_lossy()))
    }

    /// Get line and the target symbol's offset from loc
    pub fn get_line_and_offset_from_loc(
        &self,
        file: &ast::File,
        loc: &Loc,
    ) -> (String, usize, usize, usize) {
        let (start, end) = if let Loc::File(_, start, end) = loc {
            (start, end)
        } else {
            unreachable!();
        };
        let (begin_line, mut begin_column) = file.offset_to_line_column(*start);
        let (end_line, mut end_column) = file.offset_to_line_column(*end);

        let mut full_line = self.files[file.cache_no]
            .lines()
            .nth(begin_line)
            .unwrap()
            .to_owned();

        // If the loc spans across multiple lines, we concatenate them
        if begin_line != end_line {
            for i in begin_line + 1..=end_line {
                let line = self.files[file.cache_no].lines().nth(i).unwrap();
                if i == end_line {
                    end_column += full_line.len();
                }
                full_line.push_str(line);
            }
        }

        let old_size = full_line.len();
        full_line = full_line.trim_start().parse().unwrap();

        // Calculate the size of the symbol we want to highlight
        let size = end_column - begin_column;

        // Update the offset after trimming the line
        begin_column -= old_size - full_line.len();

        (full_line, begin_line, begin_column, size)
    }
}
