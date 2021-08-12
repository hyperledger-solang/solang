use super::ast::{Diagnostic, Expression, FormatArg, Namespace, Type};
use super::expression::{cast, expression};
use super::symtable::Symtable;
use crate::parser::pt;

use std::iter::Peekable;
use std::slice::Iter;
use std::str::CharIndices;

/// Resolve string format. The format string is a subset of what python/rust supports, this is mostly
/// for debugging purposes so pretty-printing should not matter.
///
/// This is essentially a format-string lexer.
pub fn string_format(
    loc: &pt::Loc,
    literals: &[pt::StringLiteral],
    args: &[pt::Expression],
    file_no: usize,
    contract_no: Option<usize>,
    function_no: Option<usize>,
    unchecked: bool,
    ns: &mut Namespace,
    symtable: &mut Symtable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Expression, ()> {
    // first resolve the arguments. We can't say anything about the format string if the args are broken
    let mut resolved_args = Vec::new();

    for arg in args {
        let expr = expression(
            arg,
            file_no,
            contract_no,
            function_no,
            ns,
            symtable,
            false,
            unchecked,
            diagnostics,
            None,
        )?;

        let ty = expr.ty();

        resolved_args.push(cast(&arg.loc(), expr, ty.deref_any(), true, ns, diagnostics).unwrap());
    }

    let mut format_iterator = FormatIterator::new(literals).peekable();

    let mut format_args = Vec::new();
    let mut string_literal = String::new();

    while let Some((loc, ch)) = format_iterator.next() {
        if ch == '}' {
            if let Some((_, '}')) = format_iterator.peek() {
                // ok, let's skip over it
                format_iterator.next();
            } else {
                diagnostics.push(Diagnostic::error(loc, String::from("unmatched ‘}’")));
                return Err(());
            }
        }

        if ch == '{' {
            if let Some((_, '{')) = format_iterator.peek() {
                // ok, let's skip over it
                format_iterator.next();
            } else {
                if !string_literal.is_empty() {
                    format_args.push((
                        FormatArg::StringLiteral,
                        Expression::BytesLiteral(loc, Type::String, string_literal.into_bytes()),
                    ));
                    string_literal = String::new();
                }

                let specifier = parse_format_specifier(loc, &mut format_iterator, diagnostics)?;

                if resolved_args.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        String::from("missing argument to format"),
                    ));
                    return Err(());
                }

                let arg = resolved_args.remove(0);
                let arg_ty = arg.ty();
                let arg_ty = arg_ty.deref_any();

                if matches!(specifier, FormatArg::Binary | FormatArg::Hex) {
                    if !matches!(arg_ty, Type::Uint(_) | Type::Int(_)) {
                        diagnostics.push(Diagnostic::error(
                            arg.loc(),
                            String::from("argument must be signed or unsigned integer type"),
                        ));
                        return Err(());
                    }
                } else if !matches!(
                    arg_ty,
                    Type::Uint(_)
                        | Type::Int(_)
                        | Type::Bytes(_)
                        | Type::Enum(_)
                        | Type::Address(_)
                        | Type::Contract(_)
                        | Type::String
                        | Type::DynamicBytes
                        | Type::Bool
                ) {
                    diagnostics.push(Diagnostic::error(
                        arg.loc(),
                        String::from(
                            "argument must be a bool, enum, address, contract, string, or bytes",
                        ),
                    ));
                    return Err(());
                }

                format_args.push((specifier, arg));
            }
        } else {
            string_literal.push(ch);
        }
    }

    if !resolved_args.is_empty() {
        diagnostics.push(Diagnostic::error(
            *loc,
            String::from("too many argument for format string"),
        ));
        return Err(());
    }

    if !string_literal.is_empty() {
        format_args.push((
            FormatArg::StringLiteral,
            Expression::BytesLiteral(*loc, Type::String, string_literal.into_bytes()),
        ));
    }

    Ok(Expression::FormatString(*loc, format_args))
}

fn parse_format_specifier(
    loc: pt::Loc,
    format_iterator: &mut Peekable<FormatIterator>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<FormatArg, ()> {
    let mut last_loc = loc;
    let arg;

    match format_iterator.next() {
        Some((_, '}')) => Ok(FormatArg::Default),
        Some((_, ':')) => {
            match format_iterator.next() {
                Some((loc, 'x')) => {
                    arg = FormatArg::Hex;
                    last_loc = loc;
                }
                Some((loc, 'b')) => {
                    last_loc = loc;

                    arg = FormatArg::Binary;
                }
                Some((_, '}')) => {
                    return Ok(FormatArg::Default);
                }
                Some((loc, ch)) => {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        format!("unexpected format char ‘{}’", ch),
                    ));
                    return Err(());
                }
                None => {
                    diagnostics.push(Diagnostic::error(
                        last_loc,
                        String::from("missing format specifier"),
                    ));
                    return Err(());
                }
            }

            match format_iterator.next() {
                Some((_, '}')) => Ok(arg),
                Some((loc, ch)) => {
                    diagnostics.push(Diagnostic::error(
                        loc,
                        format!("unexpected format char ‘{:}’, expected closing ‘}}’", ch),
                    ));
                    Err(())
                }
                None => {
                    diagnostics.push(Diagnostic::error(
                        last_loc,
                        String::from("missing closing ‘}’"),
                    ));
                    Err(())
                }
            }
        }
        Some((loc, ch)) => {
            diagnostics.push(Diagnostic::error(
                loc,
                format!("unexpected format char ‘{}’", ch),
            ));
            Err(())
        }
        None => {
            diagnostics.push(Diagnostic::error(
                last_loc,
                String::from("missing closing ‘}’"),
            ));
            Err(())
        }
    }
}

/// We need to iterate over the string literals by character, and we need the position of each character
/// Note that string literals are concatenated so it is permitted to do:
///
/// print("foo:{}" "bar:{}".format(x, y));
struct FormatIterator<'a> {
    literals: Iter<'a, pt::StringLiteral>,
    loc: pt::Loc,
    literal: CharIndices<'a>,
}

impl<'a> FormatIterator<'a> {
    fn new(literals: &'a [pt::StringLiteral]) -> Self {
        let mut literals = literals.iter();

        let sl = literals.next().expect("should be at least one entry");

        let literal = sl.string.char_indices();

        Self {
            literals,
            loc: sl.loc,
            literal,
        }
    }
}

impl<'a> Iterator for FormatIterator<'a> {
    type Item = (pt::Loc, char);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((offset, ch)) = self.literal.next() {
            let mut loc = self.loc;
            loc.0 += offset;

            return Some((loc, ch));
        }

        if let Some(sl) = self.literals.next() {
            self.loc = sl.loc;
            self.literal = sl.string.char_indices();

            if let Some((offset, ch)) = self.literal.next() {
                let mut loc = self.loc;
                loc.0 += offset;

                return Some((loc, ch));
            }
        }

        None
    }
}
