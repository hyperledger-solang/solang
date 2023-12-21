// SPDX-License-Identifier: Apache-2.0

use super::ast::{Diagnostic, Namespace, Parameter, Tag, Type};
use solang_parser::{
    doccomment::{DocComment, DocCommentTag},
    pt,
};
use std::fmt::Write;

/// Resolve the tags for a type from parsed doccomment
pub fn resolve_tags(
    file_no: usize,
    ty: &str,
    tags: &[DocComment],
    params: Option<&[Parameter<Type>]>,
    returns: Option<&[Parameter<Type>]>,
    bases: Option<Vec<usize>>,
    ns: &mut Namespace,
) -> Vec<Tag> {
    let mut res: Vec<Tag> = Vec::new();

    for c in tags.iter().flat_map(DocComment::comments) {
        let tag_loc = pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len() + 1);
        let value_loc = pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len());
        let loc = pt::Loc::File(file_no, c.tag_offset - 1, c.value_offset + c.value.len());

        match c.tag.as_str() {
            "notice" | "author" | "title" | "dev" => {
                add_tag(loc, &mut res, c);
            }
            "param" if params.is_some() => {
                let v: Vec<&str> = c.value.splitn(2, char::is_whitespace).collect();
                if v.is_empty() || v[0].is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                        "tag '@param' missing parameter name".to_string(),
                    ));
                    continue;
                }
                let name = v[0];
                let value = v.get(1).unwrap_or(&"").to_string();

                if let Some(no) = params.unwrap().iter().position(|p| p.name_as_str() == name) {
                    if let Some(other) = res.iter().find(|e| e.tag == "param" && e.no == no) {
                        // Note: solc does not detect this problem
                        ns.diagnostics.push(Diagnostic::warning_with_note(
                            loc,
                            format!("duplicate tag '@param' for '{name}'"),
                            other.loc,
                            format!("previous tag '@param' for '{name}'"),
                        ));
                    } else {
                        res.push(Tag {
                            loc,
                            tag: String::from("param"),
                            no,
                            value,
                        });
                    }
                } else if let Some(Some(no)) =
                    returns.map(|params| params.iter().position(|p| p.name_as_str() == name))
                {
                    if let Some(other) = res.iter().find(|e| e.tag == "return" && e.no == no) {
                        // Note: solc does not detect this problem
                        ns.diagnostics.push(Diagnostic::warning_with_note(
                            loc,
                            format!("duplicate tag '@param' for '{name}'"),
                            other.loc,
                            format!("previous tag '@param' for '{name}'"),
                        ));
                    } else {
                        ns.diagnostics.push(Diagnostic::warning(
                            loc,
                            format!("'@param' used in stead of '@return' for '{name}'"),
                        ));

                        res.push(Tag {
                            loc,
                            tag: String::from("return"),
                            no,
                            value,
                        });
                    }
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        value_loc,
                        format!("function parameter named '{name}' not found"),
                    ));
                }
            }
            "return" if returns.is_some() => {
                let returns = returns.unwrap();

                if returns.is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        tag_loc,
                        "tag '@return' for function with no return values".to_string(),
                    ));
                } else if returns.len() == 1 {
                    if res.iter().any(|e| e.tag == "return") {
                        ns.diagnostics.push(Diagnostic::error(
                            tag_loc,
                            "duplicate tag '@return'".to_string(),
                        ));
                    } else {
                        res.push(Tag {
                            loc,
                            tag: String::from("return"),
                            no: 0,
                            value: c.value.to_owned(),
                        });
                    }
                } else {
                    let v: Vec<&str> = c.value.splitn(2, char::is_whitespace).collect();
                    if v.is_empty() || v[0].is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            value_loc,
                            "tag '@return' missing parameter name".to_string(),
                        ));
                        continue;
                    }
                    let name = v[0];
                    let value = v.get(1).unwrap_or(&"").to_string();

                    if let Some(no) = returns
                        .iter()
                        .position(|p| p.id.as_ref().map(|id| id.name.as_str()) == Some(name))
                    {
                        if res.iter().any(|e| e.tag == "return" && e.no == no) {
                            ns.diagnostics.push(Diagnostic::error(
                                tag_loc,
                                format!("duplicate tag '@return' for '{name}'"),
                            ));
                        } else {
                            res.push(Tag {
                                loc,
                                tag: String::from("return"),
                                no,
                                value,
                            });
                        }
                    // find next unnamed return parameter without documentation tag
                    } else if let Some((no, _)) = returns.iter().enumerate().find(|(no, p)| {
                        p.id.is_none() && !res.iter().any(|e| e.tag == "return" && e.no == *no)
                    }) {
                        res.push(Tag {
                            loc,
                            tag: String::from("return"),
                            no,
                            value,
                        });
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len()),
                            format!("function return value named '{name}' not found"),
                        ));
                    }
                }
            }
            "inheritdoc" if bases.is_some() => {
                if c.value.is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        tag_loc,
                        "missing contract for tag '@inheritdoc'".to_string(),
                    ));
                } else if bases
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|contract_no| c.value == ns.contracts[*contract_no].id.name)
                {
                    res.push(Tag {
                        loc,
                        tag: String::from("inheritdoc"),
                        no: 0,
                        value: c.value.to_owned(),
                    });
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        value_loc,
                        format!("base contract '{}' not found in tag '@inheritdoc'", c.value),
                    ));
                }
            }
            _ => {
                if let Some(custom) = c.tag.strip_prefix("custom:") {
                    if custom.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            tag_loc,
                            format!("custom tag '@{}' is missing a name", c.tag),
                        ));
                    } else {
                        add_tag(loc, &mut res, c);
                    }
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        tag_loc,
                        format!("tag '@{}' is not valid for {}", c.tag, ty),
                    ));
                }
            }
        }
    }

    res
}

/// Add a new doc comment as a tag, or append to existing one
fn add_tag(loc: pt::Loc, res: &mut Vec<Tag>, doc_comment: &DocCommentTag) {
    if let Some(prev) = res.iter_mut().find(|e| e.tag == doc_comment.tag) {
        prev.value.push(' ');
        prev.value.push_str(&doc_comment.value);
    } else {
        res.push(Tag {
            loc,
            tag: doc_comment.tag.to_owned(),
            value: doc_comment.value.to_owned(),
            no: 0,
        })
    }
}

/// Render tags as plain text string
pub fn render(tags: &[Tag]) -> String {
    let mut s = String::new();

    if let Some(tag) = tags.iter().find(|e| e.tag == "title") {
        s.push_str(&tag.value.to_string());
        s.push('\n');
        s.push('\n');
    }

    if let Some(tag) = tags.iter().find(|e| e.tag == "notice") {
        s.push_str(&tag.value.to_string());
        s.push('\n');
        s.push('\n');
    }

    if let Some(tag) = tags.iter().find(|e| e.tag == "author") {
        write!(s, "Author: {}", tag.value).unwrap();
    }

    s.trim().to_owned()
}
