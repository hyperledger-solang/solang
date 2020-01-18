use parser::ast;
use serde::Serialize;

#[derive(Debug, PartialEq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug, PartialEq)]
pub enum ErrorType {
    None,
    ParserError,
    SyntaxError,
    DeclarationError,
    TypeError,
    Warning,
}

#[derive(Debug, PartialEq)]
pub struct Note {
    pub pos: ast::Loc,
    pub message: String,
}

#[derive(Debug, PartialEq)]
pub struct Output {
    pub level: Level,
    pub ty: ErrorType,
    pub pos: ast::Loc,
    pub message: String,
    pub notes: Vec<Note>,
}

impl Level {
    pub fn to_string(&self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

impl Output {
    pub fn info(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Info,
            ty: ErrorType::None,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn parser_error(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::ParserError,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn error(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn decl_error(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn type_error(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::TypeError,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning(pos: ast::Loc, message: String) -> Self {
        Output {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos,
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning_with_note(
        pos: ast::Loc,
        message: String,
        note_pos: ast::Loc,
        note: String,
    ) -> Self {
        Output {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos,
            message,
            notes: vec![Note {
                pos: note_pos,
                message: note,
            }],
        }
    }

    pub fn warning_with_notes(pos: ast::Loc, message: String, notes: Vec<Note>) -> Self {
        Output {
            level: Level::Warning,
            ty: ErrorType::Warning,
            pos,
            message,
            notes,
        }
    }

    pub fn error_with_note(
        pos: ast::Loc,
        message: String,
        note_pos: ast::Loc,
        note: String,
    ) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::None,
            pos,
            message,
            notes: vec![Note {
                pos: note_pos,
                message: note,
            }],
        }
    }

    pub fn error_with_notes(pos: ast::Loc, message: String, notes: Vec<Note>) -> Self {
        Output {
            level: Level::Error,
            ty: ErrorType::None,
            pos,
            message,
            notes,
        }
    }

    fn formated_message(&self, filename: &str, pos: &FilePostitions) -> String {
        let loc = pos.to_string(self.pos);

        let mut s = format!(
            "{}:{}: {}: {}",
            filename,
            loc,
            self.level.to_string(),
            self.message
        );

        for note in &self.notes {
            let loc = pos.to_string(note.pos);

            s.push_str(&format!(
                "\n\t{}:{}: {}: {}",
                filename, loc, "note", note.message
            ));
        }

        s
    }
}

pub fn print_messages(filename: &str, src: &str, messages: &[Output], verbose: bool) {
    let pos = FilePostitions::new(src);

    for msg in messages {
        if !verbose && msg.level == Level::Info {
            continue;
        }

        eprintln!("{}", msg.formated_message(filename, &pos));
    }
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
    pub sourceLocation: LocJson,
    #[serde(rename = "type")]
    pub ty: String,
    pub component: String,
    pub severity: String,
    pub message: String,
    pub formattedMessage: String,
}

pub fn message_as_json(filename: &str, src: &str, messages: &[Output]) -> Vec<OutputJson> {
    let mut json = Vec::new();

    let pos = FilePostitions::new(src);

    for msg in messages {
        if msg.level == Level::Info {
            continue;
        }

        json.push(OutputJson {
            sourceLocation: LocJson {
                file: filename.to_string(),
                start: msg.pos.0,
                end: msg.pos.1,
            },
            ty: format!("{:?}", msg.ty),
            component: "general".to_owned(),
            severity: msg.level.to_string().to_owned(),
            message: msg.message.to_owned(),
            formattedMessage: msg.formated_message(filename, &pos),
        });
    }

    json
}

struct FilePostitions(Vec<usize>);

impl FilePostitions {
    fn new(src: &str) -> Self {
        let mut line_starts = Vec::new();

        for (ind, c) in src.char_indices() {
            if c == '\n' {
                line_starts.push(ind);
            }
        }

        FilePostitions(line_starts)
    }

    fn to_string(&self, loc: ast::Loc) -> String {
        let (from_line, from_column) = self.convert(loc.0);
        let (to_line, to_column) = self.convert(loc.1);

        if from_line == to_line && from_column == to_column {
            format!("{}:{}", from_line, from_column)
        } else if from_line == to_line {
            format!("{}:{}-{}", from_line, from_column, to_column)
        } else {
            format!("{}:{}-{}:{}", from_line, from_column, to_line, to_column)
        }
    }

    fn convert(&self, loc: usize) -> (usize, usize) {
        let mut line_no = 1;
        let mut col_no = loc + 1;

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
