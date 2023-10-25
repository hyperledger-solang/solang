// SPDX-License-Identifier: Apache-2.0

use super::ast::{File, Namespace};
use solang_parser::pt::Loc;
use std::{fmt, path};

pub enum PathDisplay {
    None,
    Filename,
    FullPath,
}

impl File {
    pub fn new(
        path: path::PathBuf,
        contents: &str,
        cache_no: usize,
        import_no: Option<usize>,
    ) -> Self {
        let mut line_starts = Vec::new();

        for (ind, c) in contents.char_indices() {
            if c == '\n' {
                line_starts.push(ind + 1);
            }
        }

        File {
            path,
            line_starts,
            cache_no: Some(cache_no),
            import_no,
        }
    }

    /// Give a position as a human readable position
    pub fn loc_to_string(&self, display: PathDisplay, start: usize, end: usize) -> String {
        let (from_line, from_column) = self.offset_to_line_column(start);
        let (to_line, to_column) = self.offset_to_line_column(end);

        let path = match display {
            PathDisplay::None => "".to_owned(),
            PathDisplay::Filename => format!("{}:", self.file_name()),
            PathDisplay::FullPath => format!("{self}:"),
        };

        if from_line == to_line && from_column == to_column {
            format!("{}{}:{}", path, from_line + 1, from_column + 1)
        } else if from_line == to_line {
            format!(
                "{}{}:{}-{}",
                path,
                from_line + 1,
                from_column + 1,
                to_column + 1
            )
        } else {
            format!(
                "{}{}:{}-{}:{}",
                path,
                from_line + 1,
                from_column + 1,
                to_line + 1,
                to_column + 1
            )
        }
    }

    /// Convert an offset to line and column number, based zero
    pub fn offset_to_line_column(&self, loc: usize) -> (usize, usize) {
        let line_no = self
            .line_starts
            .partition_point(|line_start| loc >= *line_start);

        let col_no = if line_no > 0 {
            loc - self.line_starts[line_no - 1]
        } else {
            loc
        };

        (line_no, col_no)
    }

    /// Convert line + char to offset
    pub fn get_offset(&self, line_no: usize, column_no: usize) -> Option<usize> {
        if line_no == 0 {
            Some(column_no)
        } else {
            self.line_starts
                .get(line_no - 1)
                .map(|offset| offset + column_no)
        }
    }

    pub fn file_name(&self) -> String {
        self.path.file_name().unwrap().to_string_lossy().into()
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(not(windows))]
        let res = write!(f, "{}", self.path.display());

        #[cfg(windows)]
        let res = write!(f, "{}", fix_windows_verbatim(&self.path).display());

        res
    }
}

impl Namespace {
    /// Give a position as a human readable position
    pub fn loc_to_string(&self, display: PathDisplay, loc: &Loc) -> String {
        match loc {
            Loc::File(file_no, start, end) => {
                self.files[*file_no].loc_to_string(display, *start, *end)
            }
            Loc::Builtin => String::from("builtin"),
            Loc::Codegen => String::from("codegen"),
            Loc::Implicit => String::from("implicit"),
            Loc::CommandLine => String::from("commandline"),
        }
    }

    /// File number of the top level source unit which was compiled
    pub fn top_file_no(&self) -> usize {
        self.files
            .iter()
            .position(|file| file.cache_no.is_some())
            .unwrap()
    }
}

/// Windows verbatim paths look like \\?\C:\foo\bar which not very human readable,
/// so fix up paths. This is a copy of fn fix_windows_verbatim_for_gcc in rust
/// https://github.com/rust-lang/rust/blob/master/compiler/rustc_fs_util/src/lib.rs#L23
#[cfg(windows)]
fn fix_windows_verbatim(p: &path::Path) -> path::PathBuf {
    use std::ffi::OsString;
    let mut components = p.components();
    let prefix = match components.next() {
        Some(path::Component::Prefix(p)) => p,
        _ => return p.to_path_buf(),
    };
    match prefix.kind() {
        path::Prefix::VerbatimDisk(disk) => {
            let mut base = OsString::from(format!("{}:", disk as char));
            base.push(components.as_path());
            path::PathBuf::from(base)
        }
        path::Prefix::VerbatimUNC(server, share) => {
            let mut base = OsString::from(r"\\");
            base.push(server);
            base.push(r"\");
            base.push(share);
            base.push(components.as_path());
            path::PathBuf::from(base)
        }
        _ => p.to_path_buf(),
    }
}
