use super::ast::File;
use crate::parser::pt::Loc;
use std::path::PathBuf;

impl File {
    pub fn new(path: PathBuf, contents: &str, cache_no: usize) -> Self {
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
    pub fn loc_to_string(&self, loc: &Loc) -> String {
        let (from_line, from_column) = self.offset_to_line_column(loc.1);
        let (to_line, to_column) = self.offset_to_line_column(loc.2);

        if from_line == to_line && from_column == to_column {
            format!(
                "{}:{}:{}",
                self.path.display(),
                from_line + 1,
                from_column + 1
            )
        } else if from_line == to_line {
            format!(
                "{}:{}:{}-{}",
                self.path.display(),
                from_line + 1,
                from_column + 1,
                to_column + 1
            )
        } else {
            format!(
                "{}:{}:{}-{}:{}",
                self.path.display(),
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
