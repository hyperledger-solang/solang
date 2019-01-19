
use ast;

#[derive(Debug,PartialEq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug,PartialEq)]
pub struct Output {
    pub level: Level,
    pub pos: ast::Loc,
    pub message: String
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
    pub fn error(pos: ast::Loc, message: String) -> Self {
        Output{level: Level::Error, pos, message}
    }

    pub fn warning(pos: ast::Loc, message: String) -> Self {
        Output{level: Level::Warning, pos, message}
    }

    pub fn is_fatal(&self) -> bool {
       if let Level::Error = self.level {
           true
       } else {
           false
       }
    }
}

pub fn print_messages(filename: &str, src: &str, messages: &Vec<Output>) {
    let mut line_starts = Vec::new();

    for (ind, c) in src.char_indices() {
        if c == '\n' {
            line_starts.push(ind);
        }
    }

    for msg in messages {
        let mut line_no = 0;
        let mut col_no = 1;

        for l in &line_starts {
            if msg.pos.0 < *l {
                break;
            }

            line_no += 1;
            col_no = (msg.pos.0 - l) + 1;
        }

        eprintln!("{}:{}:{}: {}: {}", filename, line_no, col_no, msg.level.to_string(), msg.message);
    }
}
