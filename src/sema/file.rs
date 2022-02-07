use super::ast::{File, Namespace};
use crate::parser::pt::Loc;
use std::{fmt, path};

impl File {
    pub fn new(path: path::PathBuf, contents: &str, cache_no: usize) -> Self {
        let mut line_starts = Vec::new();

        for (ind, c) in contents.char_indices() {
            if c == '\n' {
                line_starts.push(ind + 1);
            }
        }

        File {
            path,
            line_starts,
            cache_no,
        }
    }

    /// Give a position as a human readable position
    pub fn loc_to_string(&self, start: usize, end: usize) -> String {
        let (from_line, from_column) = self.offset_to_line_column(start);
        let (to_line, to_column) = self.offset_to_line_column(end);

        if from_line == to_line && from_column == to_column {
            format!("{}:{}:{}", self, from_line + 1, from_column + 1)
        } else if from_line == to_line {
            format!(
                "{}:{}:{}-{}",
                self,
                from_line + 1,
                from_column + 1,
                to_column + 1
            )
        } else {
            format!(
                "{}:{}:{}-{}:{}",
                self,
                from_line + 1,
                from_column + 1,
                to_line + 1,
                to_column + 1
            )
        }
    }

    /// Convert an offset to line and column number, based zero
    pub fn offset_to_line_column(&self, loc: usize) -> (usize, usize) {
        let mut line_no = 0;
        let mut col_no = loc;

        // Here we do a linear scan. It should be possible to do binary search
        for l in &self.line_starts {
            if loc < *l {
                break;
            }

            if loc == *l {
                col_no -= 1;
                break;
            }

            col_no = loc - l;

            line_no += 1;
        }

        (line_no, col_no)
    }

    /// Convert line + char to offset
    pub fn get_offset(&self, line_no: usize, column_no: usize) -> usize {
        if line_no == 0 {
            column_no
        } else {
            self.line_starts[line_no - 1] + column_no
        }
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
    pub fn loc_to_string(&self, loc: &Loc) -> String {
        match loc {
            Loc::File(file_no, start, end) => self.files[*file_no].loc_to_string(*start, *end),
            Loc::Builtin => String::from("builtin"),
            Loc::Codegen => String::from("codegen"),
            Loc::Implicit => String::from("implicit"),
            Loc::CommandLine => String::from("commandline"),
        }
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
