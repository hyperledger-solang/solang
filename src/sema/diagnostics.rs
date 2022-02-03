use super::ast::{Diagnostic, Level, Namespace};
use crate::file_resolver::FileResolver;
use codespan_reporting::{diagnostic, files, term};
use serde::Serialize;
use std::{io, sync::Arc};

/// Print the diagnostics to stderr with fancy formatting
pub fn print_diagnostics(cache: &FileResolver, ns: &Namespace, debug: bool) {
    let (files, file_id) = convert_files(ns, cache);

    let writer = term::termcolor::StandardStream::stderr(term::termcolor::ColorChoice::Always);
    let config = term::Config::default();

    for msg in &ns.diagnostics {
        if msg.level == Level::Debug && !debug {
            continue;
        }

        let diagnostic = convert_diagnostic(msg, &file_id);

        term::emit(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
    }
}

/// Print the diagnostics to stdout with plain formatting
pub fn print_diagnostics_plain(cache: &FileResolver, ns: &Namespace, debug: bool) {
    let (files, file_id) = convert_files(ns, cache);

    let config = term::Config::default();

    for msg in &ns.diagnostics {
        if msg.level == Level::Debug && !debug {
            continue;
        }

        let diagnostic = convert_diagnostic(msg, &file_id);

        let mut buffer = RawBuffer::new();

        term::emit(&mut buffer, &config, &files, &diagnostic).unwrap();

        println!("{}", buffer.into_string());
    }
}

/// Do we have any errors
pub fn any_errors(diagnotic: &[Diagnostic]) -> bool {
    diagnotic.iter().any(|m| m.level == Level::Error)
}

fn convert_diagnostic(msg: &Diagnostic, file_id: &[usize]) -> diagnostic::Diagnostic<usize> {
    let diagnostic = diagnostic::Diagnostic::new(match msg.level {
        Level::Debug => diagnostic::Severity::Help,
        Level::Info => diagnostic::Severity::Note,
        Level::Error => diagnostic::Severity::Error,
        Level::Warning => diagnostic::Severity::Warning,
    })
    .with_message(msg.message.to_owned());

    let mut labels = Vec::new();

    if let Some(pos) = msg.pos {
        labels.push(diagnostic::Label::primary(file_id[pos.0], pos.1..pos.2));
    }

    for note in &msg.notes {
        labels.push(
            diagnostic::Label::secondary(file_id[note.pos.0], note.pos.1..note.pos.2)
                .with_message(note.message.to_owned()),
        );
    }

    if labels.is_empty() {
        diagnostic
    } else {
        diagnostic.with_labels(labels)
    }
}

fn convert_files(
    ns: &Namespace,
    cache: &FileResolver,
) -> (files::SimpleFiles<String, Arc<str>>, Vec<usize>) {
    let mut files = files::SimpleFiles::new();
    let mut file_id = Vec::new();

    for file in &ns.files {
        let (contents, _) = cache.get_file_contents_and_number(&file.path);
        file_id.push(files.add(format!("{}", file), contents.to_owned()));
    }

    (files, file_id)
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

pub fn diagnostics_as_json(ns: &Namespace, cache: &FileResolver) -> Vec<OutputJson> {
    let (files, file_id) = convert_files(ns, cache);
    let mut json = Vec::new();

    let config = term::Config {
        display_style: term::DisplayStyle::Short,
        ..Default::default()
    };

    for msg in &ns.diagnostics {
        if msg.level == Level::Info || msg.level == Level::Debug {
            continue;
        }

        let diagnostic = convert_diagnostic(msg, &file_id);

        let mut buffer = RawBuffer::new();

        term::emit(&mut buffer, &config, &files, &diagnostic).unwrap();

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
            formattedMessage: buffer.into_string(),
        });
    }

    json
}

pub struct RawBuffer {
    buf: Vec<u8>,
}

impl RawBuffer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RawBuffer {
        RawBuffer { buf: Vec::new() }
    }

    pub fn into_string(self) -> String {
        String::from_utf8(self.buf).unwrap()
    }
}

impl io::Write for RawBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl term::termcolor::WriteColor for RawBuffer {
    fn supports_color(&self) -> bool {
        false
    }

    fn set_color(&mut self, _: &term::termcolor::ColorSpec) -> io::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        Ok(())
    }
}
