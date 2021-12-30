use super::ast::{Diagnostic, ErrorType, Level, Namespace, Note};
use crate::file_resolver::FileResolver;
use crate::parser::pt::Loc;
use serde::Serialize;

fn formatted_message(diagnostic: &Diagnostic, ns: &Namespace, cache: &FileResolver) -> String {
    let mut s = if let Some(pos) = diagnostic.pos {
        let loc = ns.files[pos.0].loc_to_string(&pos);

        let (full_line, beg_line_no, beg_offset, type_size) =
            cache.get_line_and_offset_from_loc(&ns.files[pos.0], &pos);

        format!(
            "{}: {}: {}\nLine {}:\n\t{}\n\t{:-<7$}{:^<8$}",
            loc,
            diagnostic.level.to_string(),
            diagnostic.message,
            beg_line_no + 1,
            full_line,
            "",
            "",
            beg_offset,
            type_size
        )
    } else {
        format!("solang: {}: {}", diagnostic.level.to_string(), diagnostic.message)
    };

    for note in &diagnostic.notes {
        let loc = ns.files[note.pos.0].loc_to_string(&note.pos);

        let (full_line, beg_line_no, beg_offset, type_size) =
            cache.get_line_and_offset_from_loc(&ns.files[note.pos.0], &note.pos);

        s.push_str(&format!(
            "\n\t{}: {}: {}\n\tLine {}:\n\t\t{}\n\t\t{:-<7$}{:^<8$}",
            loc,
            "note",
            note.message,
            beg_line_no + 1,
            full_line,
            "",
            "",
            beg_offset,
            type_size
        ));
    }

    s
}

pub fn print_messages(cache: &FileResolver, ns: &Namespace, debug: bool) {
    for msg in &ns.diagnostics {
        if !debug && msg.level == Level::Debug {
            continue;
        }

        eprintln!("{}", formatted_message(msg, ns, cache));
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

pub fn message_as_json(ns: &Namespace, cache: &FileResolver) -> Vec<OutputJson> {
    let mut json = Vec::new();

    for msg in &ns.diagnostics {
        if msg.level == Level::Info || msg.level == Level::Debug {
            continue;
        }

        let location = msg.pos.map(|pos| LocJson {
            file: format!("{}", ns.files[pos.0].path.display()),
            start: pos.1 + 1,
            end: pos.2 + 1,
        });

        json.push(OutputJson {
            sourceLocation: location,
            ty: format!("{:?}", msg.ty),
            component: "general".to_owned(),
            severity: msg.level.to_string().to_owned(),
            message: msg.message.to_owned(),
            formattedMessage: formatted_message(msg, ns, cache),
        });
    }

    json
}
