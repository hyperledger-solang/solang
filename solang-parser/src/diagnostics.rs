// SPDX-License-Identifier: Apache-2.0

//! Solidity parser diagnostics.

use crate::pt;
use crate::pt::Loc;
use std::fmt;

/// The level of a diagnostic.
#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Level {
    /// Debug diagnostic level.
    Debug,
    /// Info diagnostic level.
    Info,
    /// Warning diagnostic level.
    Warning,
    /// Error diagnostic level.
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Level {
    /// Returns this type as a static string slice.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

/// The type of a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    /// No specific error type.
    None,
    /// Parser error.
    ParserError,
    /// Syntax error.
    SyntaxError,
    /// Declaration error.
    DeclarationError,
    /// Cast error.
    CastError,
    /// Type error.
    TypeError,
    /// Warning.
    Warning,
}

/// A diagnostic note.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    /// The code location of the note.
    pub loc: pt::Loc,
    /// The message of the note.
    pub message: String,
}

/// A Solidity diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Diagnostic {
    /// The code location of the diagnostic.
    pub loc: pt::Loc,
    /// The level of the diagnostic.
    pub level: Level,
    /// The type of diagnostic.
    pub ty: ErrorType,
    /// The message of the diagnostic.
    pub message: String,
    /// Extra notes about the diagnostic.
    pub notes: Vec<Note>,
}

impl Diagnostic {
    /// Instantiate a new Diagnostic with the given location and message at the debug level.
    pub fn debug(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Debug,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new Diagnostic with the given location and message at the info level.
    pub fn info(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Info,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new parser error Diagnostic.
    pub fn parser_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::ParserError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new syntax error Diagnostic.
    pub fn error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new declaration error Diagnostic.
    pub fn decl_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new cast error error Diagnostic.
    pub fn cast_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new cast error error Diagnostic, with a note.
    pub fn cast_error_with_note(loc: Loc, message: String, note_loc: Loc, note: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    /// Instantiate a new type error error Diagnostic.
    pub fn type_error(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new cast error Diagnostic at the warning level.
    pub fn cast_warning(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new warning Diagnostic.
    pub fn warning(loc: Loc, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Instantiate a new warning Diagnostic, with a note.
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

    /// Instantiate a new warning Diagnostic, with multiple notes.
    pub fn warning_with_notes(loc: Loc, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes,
        }
    }

    /// Instantiate a new error Diagnostic, with a note.
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

    /// Instantiate a new error Diagnostic, with multiple notes.
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
