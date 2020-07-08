use output::Output;
use parser::parse;
use parser::pt::{Loc, SourceUnit};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

//
// When compiling multiple solidity files, the same file might be import
// multiple times. We should cache the text of the file and the parse tree,
// if the file parsed correctly.
//
pub struct ParsedFile {
    pub source_code: Rc<String>,
    pub parse_tree: Option<Rc<SourceUnit>>,
}

pub struct ParsedCache {
    import_path: Vec<PathBuf>,
    // Find file by how they are identified in code
    by_name: HashMap<String, usize>,
    // Find file by operating system path
    by_path: HashMap<PathBuf, usize>,
    // The actual file cache
    files: Vec<ParsedFile>,
}

impl Default for ParsedCache {
    fn default() -> Self {
        ParsedCache::new()
    }
}

impl ParsedCache {
    /// Create a new file cache object
    pub fn new() -> Self {
        ParsedCache {
            import_path: Vec::new(),
            by_name: HashMap::new(),
            by_path: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Add import path. This should be the canonicalized path
    pub fn add_import_path(&mut self, path: PathBuf) {
        self.import_path.push(path);
    }

    /// Update the cache for the filename with the given contents
    pub fn set_file_contents(&mut self, filename: String, contents: String) {
        let pos = self.files.len();

        self.files.push(ParsedFile {
            source_code: Rc::new(contents),
            parse_tree: None,
        });

        self.by_name.insert(filename, pos);
    }

    /// Get file with contents. This must be a file which was previously
    /// add to the cache
    pub fn get_file_contents(&mut self, filename: &str) -> Rc<String> {
        let pos = self
            .by_name
            .get(filename)
            .expect("file should exist in cache already");

        self.files[*pos].source_code.clone()
    }

    /// Parse the given file. Return cached parse tree if available; else read file
    /// and parse.
    pub fn parse(&mut self, filename: &str) -> Result<Rc<SourceUnit>, Vec<Output>> {
        let file = match self.by_name.get(filename) {
            Some(pos) => *pos,
            None => {
                if let Some(path) = self.resolve_file(filename) {
                    if let Some(pos) = self.by_path.get(&path) {
                        // we found a different name for the same file
                        self.by_name.insert(filename.to_string(), *pos);

                        *pos
                    } else {
                        // read the file
                        let mut f = match File::open(&path) {
                            Err(err_info) => {
                                return Err(vec![Output::error(
                                    Loc(0, 0),
                                    format!(
                                        "cannot open file ‘{}’: {}",
                                        filename,
                                        err_info.to_string()
                                    ),
                                )]);
                            }
                            Ok(file) => file,
                        };

                        let mut contents = String::new();
                        if let Err(e) = f.read_to_string(&mut contents) {
                            return Err(vec![Output::error(
                                Loc(0, 0),
                                format!("failed to read file ‘{}’: {}", filename, e.to_string()),
                            )]);
                        }

                        let pos = self.files.len();

                        self.files.push(ParsedFile {
                            source_code: Rc::new(contents),
                            parse_tree: None,
                        });

                        self.by_name.insert(filename.to_string(), pos);
                        self.by_path.insert(path, pos);

                        pos
                    }
                } else {
                    return Err(vec![Output::error(
                        Loc(0, 0),
                        format!("file not found ‘{}’", filename),
                    )]);
                }
            }
        };

        if let Some(pt) = &self.files[file].parse_tree {
            return Ok(pt.clone());
        }

        let pt = Rc::new(parse(&self.files[file].source_code)?);

        self.files[file].parse_tree = Some(pt.clone());

        Ok(pt)
    }

    /// Walk the import path to search for a file. If no import path is set up,
    /// return. Check each import path if the file can be found in a subdirectory
    /// of that path, and return the canonicalized path.
    fn resolve_file(&self, filename: &str) -> Option<PathBuf> {
        let path = PathBuf::from(filename);

        for i in &self.import_path {
            // we want to prevent walking up the tree with .. or /
            if let Ok(p) = i.join(path.clone()).canonicalize() {
                // we want to prevent walking up the tree with .. or /
                if p.starts_with(i) {
                    return Some(p);
                }
            }
        }

        None
    }
}
