use crate::pt;
use crate::pt::Loc;

#[derive(Debug, Eq, Hash, PartialOrd, Ord, PartialEq)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

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

#[derive(Debug, Eq, Hash, PartialOrd, Ord, PartialEq)]
pub enum ErrorType {
    None,
    ParserError,
    SyntaxError,
    DeclarationError,
    TypeError,
    Warning,
}

#[derive(Debug, Eq, Hash, PartialOrd, Ord, PartialEq)]
pub struct Note {
    pub loc: pt::Loc,
    pub message: String,
}

#[derive(Debug, Eq, Hash, PartialOrd, Ord, PartialEq)]
pub struct Diagnostic {
    pub loc: pt::Loc,
    pub level: Level,
    pub ty: ErrorType,
    pub message: String,
    pub notes: Vec<Note>,
}

impl Diagnostic {
    pub fn debug(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Debug,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn info(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Info,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn parser_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::ParserError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn decl_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn type_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning_with_note(loc: Loc, message: String, note_loc: Loc, note: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    pub fn warning_with_notes(loc: Loc, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes,
        }
    }

    pub fn error_with_note(loc: Loc, message: String, note_loc: Loc, note: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    pub fn error_with_notes(loc: Loc, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            loc,
            message,
            notes,
        }
    }
}
