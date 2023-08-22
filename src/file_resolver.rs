// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast;
use normalize_path::NormalizePath;
use solang_parser::pt::Loc;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Default)]
pub struct FileResolver {
    /// Set of import paths search for imports
    import_paths: Vec<(Option<OsString>, PathBuf)>,
    /// List file by path
    cached_paths: HashMap<PathBuf, usize>,
    /// The actual file contents
    files: Vec<ResolvedFile>,
}

/// When we resolve a file, we need to know its base compared to the import so
/// we can resolve the next import, and the full path on the filesystem.
/// Since the same filename can exists in multiple imports, we need to tell the
/// user exactly which file has errors/warnings.
#[derive(Clone, Debug)]
pub struct ResolvedFile {
    /// Original name used on cli or import statement
    pub path: OsString,
    /// Full path on the filesystem
    pub full_path: PathBuf,
    /// Which import path was used, if any
    pub import_no: Option<usize>,
    /// The actual file contents
    pub contents: Arc<str>,
}

impl FileResolver {
    /// Add import path
    pub fn add_import_path(&mut self, path: &Path) {
        self.import_paths.push((None, path.to_path_buf()));
    }

    /// Add import map
    pub fn add_import_map(&mut self, map: OsString, path: PathBuf) {
        self.import_paths.push((Some(map), path));
    }

    /// Get the import path and the optional mapping corresponding to `import_no`.
    pub fn get_import_path(&self, import_no: usize) -> Option<&(Option<OsString>, PathBuf)> {
        self.import_paths.get(import_no)
    }

    /// Get the import path corresponding to a map
    pub fn get_import_map(&self, map: &OsString) -> Option<&PathBuf> {
        self.import_paths
            .iter()
            .find(|(m, _)| m.as_ref() == Some(map))
            .map(|(_, pb)| pb)
    }

    /// Update the cache for the filename with the given contents
    pub fn set_file_contents(&mut self, path: &str, contents: String) {
        let pos = self.files.len();

        let pathbuf = PathBuf::from(path);

        self.files.push(ResolvedFile {
            path: path.into(),
            full_path: pathbuf.clone(),
            contents: Arc::from(contents),
            import_no: None,
        });

        self.cached_paths.insert(pathbuf, pos);
    }

    /// Get the file contents of `file_no`th file if it exists
    pub fn get_contents_of_file_no(&self, file_no: usize) -> Option<Arc<str>> {
        self.files.get(file_no).map(|f| f.contents.clone())
    }

    /// Get file with contents. This must be a file which was previously
    /// add to the cache
    pub fn get_file_contents_and_number(&self, file: &Path) -> (Arc<str>, usize) {
        let file_no = self.cached_paths[file];

        (self.files[file_no].contents.clone(), file_no)
    }

    /// Atempt to resolve a file, either from the cache or from the filesystem.
    /// Returns Ok(Some(..)) if the file is found and loaded
    /// Returns Ok(None) if no file by this path can be found.
    /// Returns Err(..) if a file was found but could not be read.
    fn try_file(
        &mut self,
        filename: &OsStr,
        path: &Path,
        import_no: Option<usize>,
    ) -> Result<Option<ResolvedFile>, String> {
        // For accessing the cache, remove "." and ".." path components
        let cache_path = path.normalize();

        if let Some(cache) = self.cached_paths.get(&cache_path) {
            let mut file = self.files[*cache].clone();
            file.import_no = import_no;
            return Ok(Some(file));
        }

        if let Ok(full_path) = path.canonicalize() {
            let file = self.load_file(filename, &full_path, import_no)?;
            return Ok(Some(file.clone()));
        }

        Ok(None)
    }

    /// Populate the cache with absolute file path
    fn load_file(
        &mut self,
        filename: &OsStr,
        path: &Path,
        import_no: Option<usize>,
    ) -> Result<&ResolvedFile, String> {
        if let Some(cache) = self.cached_paths.get(path) {
            if self.files[*cache].import_no == import_no {
                return Ok(&self.files[*cache]);
            }
        }

        // read the file
        let mut f = match File::open(path) {
            Err(err_info) => {
                return Err(format!(
                    "cannot open file '{}': {}",
                    path.display(),
                    err_info
                ));
            }
            Ok(file) => file,
        };

        let mut contents = String::new();
        if let Err(e) = f.read_to_string(&mut contents) {
            return Err(format!("failed to read file '{}': {}", path.display(), e));
        }

        let pos = self.files.len();

        self.files.push(ResolvedFile {
            path: filename.into(),
            full_path: path.to_path_buf(),
            import_no,
            contents: Arc::from(contents),
        });

        self.cached_paths.insert(path.to_path_buf(), pos);

        Ok(&self.files[pos])
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

        // XXX: This is a HACK! There is a bug where caching currently happens
        // over canonicalized path names instead of VFS path names. My updates
        // to bring solang import resolution in line with solc broke something
        // with caching.
        //
        // If this file has already been cashed in the VFS, return it.
        if let Some(import_no) = self.cached_paths.get(&path) {
            if let Some(res) = self.files.get(*import_no) {
                return Ok((*res).clone());
            } else {
                return Err("".to_string());
            }
        }

        let mut result: Vec<Result<ResolvedFile, String>> = vec![];

        // Only when the path starts with ./ or ../ are relative paths considered; this means
        // that `import "b.sol";` will check the import paths for b.sol, while `import "./b.sol";`
        // will only the path relative to the current file.
        if path.starts_with("./") || path.starts_with("../") {
            if let Some(ResolvedFile {
                import_no,
                full_path,
                ..
            }) = parent
            {
                let curdir = PathBuf::from(".");
                let base = full_path.parent().unwrap_or(&curdir);
                let path = base.join(&path);

                if let Some(file) = self.try_file(filename, &path, *import_no)? {
                    // No ambiguity possible, so just return
                    return Ok(file);
                }
            }

            return Err(format!("file not found '{}'", filename.to_string_lossy()));
        }

        if parent.is_none() {
            if let Some(file) = self.try_file(filename, &path, None)? {
                return Ok(file);
            } else if path.is_absolute() {
                return Err(format!("file not found '{}'", filename.to_string_lossy()));
            }
        }

        // first check maps

        let mut remapped = path.clone();

        for import_map_no in 0..self.import_paths.len() {
            if let (Some(mapping), target) = &self.import_paths[import_map_no].clone() {
                if let Ok(relpath) = path.strip_prefix(mapping) {
                    remapped = target.join(relpath);
                }
            }
        }

        let path = remapped;

        // walk over the import paths until we find one that resolves
        for import_no in 0..self.import_paths.len() {
            if let (None, import_path) = &self.import_paths[import_no] {
                let path = import_path.join(&path);

                if let Some(file) = self.try_file(filename, &path, Some(import_no))? {
                    result.push(Ok(file));
                }
            }
        }

        if result.is_empty() {
            Err(format!("file not found '{}'", filename.to_string_lossy()))
        } else if result.len() > 1 {
            let filepaths = result
                .iter()
                .flatten()
                .map(|f| &f.full_path)
                .collect::<Vec<&PathBuf>>();

            let mut filenames = vec![];
            for fp in filepaths {
                filenames.push(fp.to_str().unwrap().to_string());
            }

            Err(format!(
                "found multiple files matching '{}'\n- {}",
                filename.to_string_lossy(),
                filenames.join("\n- ")
            ))
        } else {
            result.pop().unwrap()
        }
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
        let cache_no = file.cache_no.unwrap();
        let (begin_line, mut begin_column) = file.offset_to_line_column(*start);
        let (end_line, mut end_column) = file.offset_to_line_column(*end);

        let mut full_line = self.files[cache_no]
            .contents
            .lines()
            .nth(begin_line)
            .unwrap()
            .to_owned();

        // If the loc spans across multiple lines, we concatenate them
        if begin_line != end_line {
            for i in begin_line + 1..=end_line {
                let line = self.files[cache_no].contents.lines().nth(i).unwrap();
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
