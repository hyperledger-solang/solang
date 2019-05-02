
use ast;

#[derive(Debug,PartialEq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug,PartialEq)]
pub struct Note {
    pub pos: ast::Loc,
    pub message: String
}

#[derive(Debug,PartialEq)]
pub struct Output {
    pub level: Level,
    pub pos: ast::Loc,
    pub message: String,
    pub notes: Vec<Note>
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
        Output{level: Level::Info, pos, message, notes: Vec::new()}
    }

    pub fn error(pos: ast::Loc, message: String) -> Self {
        Output{level: Level::Error, pos, message, notes: Vec::new()}
    }

    pub fn warning(pos: ast::Loc, message: String) -> Self {
        Output{level: Level::Warning, pos, message, notes: Vec::new()}
    }

    pub fn error_with_note(pos: ast::Loc, message: String, note_pos: ast::Loc, note: String) -> Self {
        Output{level: Level::Error, pos, message, notes: vec!(Note{pos: note_pos, message: note})}
    }

    pub fn error_with_notes(pos: ast::Loc, message: String, notes: Vec<Note>) -> Self {
        Output{level: Level::Error, pos, message, notes}
    }
}

pub fn print_messages(filename: &str, src: &str, messages: &Vec<Output>, verbose: bool) {
    let mut line_starts = Vec::new();

    for (ind, c) in src.char_indices() {
        if c == '\n' {
            line_starts.push(ind);
        }
    }

    let convert_loc = |loc| {
        let mut line_no = 1;
        let mut col_no = loc + 1;

        for l in &line_starts {
            if loc < *l {
                break;
            }

            line_no += 1;
            col_no = loc - l;
        }

        (line_no, col_no)
    };

    for msg in messages {
        if !verbose && msg.level == Level::Info {
            continue;
        }

        let mut loc = convert_loc(msg.pos.0);

        eprintln!("{}:{}:{}: {}: {}", filename, loc.0, loc.1, msg.level.to_string(), msg.message);

        for note in &msg.notes {
            let mut loc = convert_loc(note.pos.0);

            eprintln!("{}:{}:{}: {}: {}", filename, loc.0, loc.1, "note", note.message);
        }
    }
}
