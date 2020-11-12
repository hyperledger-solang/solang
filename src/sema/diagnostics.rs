use super::ast::{Diagnostic, ErrorType, Level, Namespace, Note};
use crate::file_cache::FileCache;
use crate::parser::pt::Loc;
use serde::Serialize;

impl Level {
    pub fn to_string(&self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

impl Diagnostic {
    pub fn debug(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Debug,
            ty: ErrorType::None,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn info(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Info,
            ty: ErrorType::None,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn parser_error(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::ParserError,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn error(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn decl_error(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn type_error(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning(pos: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos: Some(pos),
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning_with_note(pos: Loc, message: String, note_pos: Loc, note: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos: Some(pos),
            message,
            notes: vec![Note {
                pos: note_pos,
                message: note,
            }],
        }
    }

    pub fn warning_with_notes(pos: Loc, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos: Some(pos),
            message,
            notes,
        }
    }

    pub fn error_with_note(pos: Loc, message: String, note_pos: Loc, note: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            pos: Some(pos),
            message,
            notes: vec![Note {
                pos: note_pos,
                message: note,
            }],
        }
    }

    pub fn error_with_notes(pos: Loc, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            pos: Some(pos),
            message,
            notes,
        }
    }

    fn formated_message(&self, filename: &str, offset_converter: &OffsetToLineColumn) -> String {
        let mut s = if let Some(pos) = self.pos {
            let loc = offset_converter.to_string(pos);

            format!(
                "{}:{}: {}: {}",
                filename,
                loc,
                self.level.to_string(),
                self.message
            )
        } else {
            format!("solang: {}: {}", self.level.to_string(), self.message)
        };

        for note in &self.notes {
            let loc = offset_converter.to_string(note.pos);

            s.push_str(&format!(
                "\n\t{}:{}: {}: {}",
                filename, loc, "note", note.message
            ));
        }

        s
    }
}

pub fn print_messages(cache: &mut FileCache, ns: &Namespace, debug: bool) {
    let mut current_file_no = None;
    let mut offset_converter = OffsetToLineColumn(Vec::new());
    let mut filename = "";

    for msg in &ns.diagnostics {
        if !debug && msg.level == Level::Debug {
            continue;
        }

        let file_no = msg.pos.map(|pos| pos.0);

        if file_no != current_file_no {
            filename = &ns.files[file_no.unwrap()];

            offset_converter = OffsetToLineColumn::new(&*cache.get_file_contents(filename));
            current_file_no = file_no;
        }

        eprintln!("{}", msg.formated_message(filename, &offset_converter));
    }
}

/// Do we have any errors
pub fn any_errors(diagnotic: &[Diagnostic]) -> bool {
    diagnotic.iter().any(|m| m.level == Level::Error)
}

#[derive(Serialize)]
pub struct LocJson {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct OutputJson {
    pub sourceLocation: Option<LocJson>,
    #[serde(rename = "type")]
    pub ty: String,
    pub component: String,
    pub severity: String,
    pub message: String,
    pub formattedMessage: String,
}

pub fn message_as_json(cache: &mut FileCache, ns: &Namespace) -> Vec<OutputJson> {
    let mut json = Vec::new();

    let mut current_file_no = None;
    let mut offset_converter = OffsetToLineColumn(Vec::new());
    let mut filename = "";

    for msg in &ns.diagnostics {
        if msg.level == Level::Info {
            continue;
        }

        let file_no = msg.pos.map(|pos| pos.0);

        if file_no != current_file_no {
            filename = &ns.files[file_no.unwrap()];

            offset_converter = OffsetToLineColumn::new(&*cache.get_file_contents(filename));
            current_file_no = file_no;
        }

        let loc_json = if let Some(pos) = msg.pos {
            Some(LocJson {
                file: filename.to_string(),
                start: pos.1,
                end: pos.2,
            })
        } else {
            None
        };

        json.push(OutputJson {
            sourceLocation: loc_json,
            ty: format!("{:?}", msg.ty),
            component: "general".to_owned(),
            severity: msg.level.to_string().to_owned(),
            message: msg.message.to_owned(),
            formattedMessage: msg.formated_message(filename, &offset_converter),
        });
    }

    json
}

/// Convert byte offset in file to line and column number
pub struct OffsetToLineColumn(Vec<usize>);

impl OffsetToLineColumn {
    /// Create a new mapping for offset to position.
    pub fn new(src: &str) -> Self {
        let mut line_starts = Vec::new();

        for (ind, c) in src.char_indices() {
            if c == '\n' {
                line_starts.push(ind);
            }
        }

        OffsetToLineColumn(line_starts)
    }

    /// Give a position as a human readable position
    pub fn to_string(&self, loc: Loc) -> String {
        let (from_line, from_column) = self.convert(loc.1);
        let (to_line, to_column) = self.convert(loc.2);

        if from_line == to_line && from_column == to_column {
            format!("{}:{}", from_line, from_column)
        } else if from_line == to_line {
            format!("{}:{}-{}", from_line, from_column, to_column)
        } else {
            format!("{}:{}-{}:{}", from_line, from_column, to_line, to_column)
        }
    }

    /// Convert an offset to line and column number
    pub fn convert(&self, loc: usize) -> (usize, usize) {
        let mut line_no = 1;
        let mut col_no = loc + 1;

        // Here we do a linear scan. It should be possible to do binary search
        for l in &self.0 {
            if loc < *l {
                break;
            }

            line_no += 1;
            col_no = loc - l;
        }

        (line_no, col_no)
    }
}
