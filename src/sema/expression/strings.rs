// SPDX-License-Identifier: Apache-2.0

use crate::sema::diagnostics::Diagnostics;
use solang_parser::diagnostics::Diagnostic;
use solang_parser::pt;

/// Unescape a string literal
pub(crate) fn unescape(
    literal: &str,
    start: usize,
    file_no: usize,
    diagnostics: &mut Diagnostics,
) -> (bool, Vec<u8>) {
    let mut s: Vec<u8> = Vec::new();
    let mut indeces = literal.char_indices();
    let mut valid = true;

    while let Some((_, ch)) = indeces.next() {
        if ch != '\\' {
            let mut buffer = [0; 4];
            s.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            continue;
        }

        match indeces.next() {
            Some((_, '\n')) => (),
            Some((_, '\\')) => s.push(b'\\'),
            Some((_, '\'')) => s.push(b'\''),
            Some((_, '"')) => s.push(b'"'),
            Some((_, 'b')) => s.push(b'\x08'),
            Some((_, 'f')) => s.push(b'\x0c'),
            Some((_, 'n')) => s.push(b'\n'),
            Some((_, 'r')) => s.push(b'\r'),
            Some((_, 't')) => s.push(b'\t'),
            Some((_, 'v')) => s.push(b'\x0b'),
            Some((i, 'x')) => match get_digits(&mut indeces, 2) {
                Ok(ch) => s.push(ch as u8),
                Err(offset) => {
                    valid = false;
                    diagnostics.push(Diagnostic::error(
                        pt::Loc::File(
                            file_no,
                            start + i,
                            start + std::cmp::min(literal.len(), offset),
                        ),
                        "\\x escape should be followed by two hex digits".to_string(),
                    ));
                }
            },
            Some((i, 'u')) => match get_digits(&mut indeces, 4) {
                Ok(codepoint) => match char::from_u32(codepoint) {
                    Some(ch) => {
                        let mut buffer = [0; 4];
                        s.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                    }
                    None => {
                        valid = false;
                        diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, start + i, start + i + 6),
                            "Found an invalid unicode character".to_string(),
                        ));
                    }
                },
                Err(offset) => {
                    valid = false;
                    diagnostics.push(Diagnostic::error(
                        pt::Loc::File(
                            file_no,
                            start + i,
                            start + std::cmp::min(literal.len(), offset),
                        ),
                        "\\u escape should be followed by four hex digits".to_string(),
                    ));
                }
            },
            Some((i, ch)) => {
                valid = false;
                diagnostics.push(Diagnostic::error(
                    pt::Loc::File(file_no, start + i, start + i + ch.len_utf8()),
                    format!("unknown escape character '{ch}'"),
                ));
            }
            None => unreachable!(),
        }
    }

    (valid, s)
}

/// Get the hex digits for an escaped \x or \u. Returns either the value or
/// or the offset of the last character
pub(super) fn get_digits(input: &mut std::str::CharIndices, len: usize) -> Result<u32, usize> {
    let mut n: u32 = 0;
    let offset;

    for _ in 0..len {
        if let Some((_, ch)) = input.next() {
            if let Some(v) = ch.to_digit(16) {
                n = (n << 4) + v;
                continue;
            }
            offset = match input.next() {
                Some((i, _)) => i,
                None => usize::MAX,
            };
        } else {
            offset = usize::MAX;
        }

        return Err(offset);
    }

    Ok(n)
}
