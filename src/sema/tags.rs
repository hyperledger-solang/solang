use super::ast::{Diagnostic, Namespace, Parameter, Tag};
use crate::parser::pt;

/// Resolve the tags for a type
pub fn resolve_tags(
    file_no: usize,
    ty: &str,
    doc: &[pt::DocComment],
    params: Option<&[Parameter]>,
    returns: Option<&[Parameter]>,
    bases: Option<&[&str]>,
    ns: &mut Namespace,
) -> Vec<Tag> {
    let mut res: Vec<Tag> = Vec::new();

    for c in doc.iter() {
        for single_c in c.comments() {
            match single_c.tag.as_str() {
                "notice" | "author" | "title" | "dev" => {
                    // fold fields with the same name
                    if let Some(prev) = res.iter_mut().find(|e| e.tag == single_c.tag) {
                        prev.value.push(' ');
                        prev.value.push_str(&single_c.value);
                    } else {
                        res.push(Tag {
                            tag: single_c.tag.to_owned(),
                            value: single_c.value.to_owned(),
                            no: 0,
                        })
                    }
                }
                "param" if params.is_some() => {
                    let v: Vec<&str> = single_c.value.splitn(2, char::is_whitespace).collect();
                    if v.is_empty() || v[0].is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(
                                file_no,
                                single_c.offset,
                                single_c.offset + single_c.tag.len(),
                            ),
                            "tag ‘@param’ missing parameter name".to_string(),
                        ));
                        continue;
                    }
                    let name = v[0];
                    let value = v.get(1).unwrap_or(&"").to_string();

                    if let Some(no) = params.unwrap().iter().position(|p| p.name == name) {
                        if res.iter().any(|e| e.tag == "param" && e.no == no) {
                            ns.diagnostics.push(Diagnostic::error(
                                pt::Loc(
                                    file_no,
                                    single_c.offset,
                                    single_c.offset + single_c.tag.len(),
                                ),
                                format!("duplicate tag ‘@param’ for ‘{}’", name),
                            ));
                        } else {
                            res.push(Tag {
                                tag: String::from("param"),
                                no,
                                value,
                            });
                        }
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(
                                file_no,
                                single_c.offset,
                                single_c.offset + single_c.tag.len(),
                            ),
                            format!("tag ‘@param’ no field ‘{}’", name),
                        ));
                    }
                }
                "return" if returns.is_some() => {
                    let returns = returns.unwrap();

                    if returns.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(
                                file_no,
                                single_c.offset,
                                single_c.offset + single_c.tag.len(),
                            ),
                            "tag ‘@return’ for function with no return values".to_string(),
                        ));
                    } else if returns.len() == 1 {
                        if res.iter().any(|e| e.tag == "return") {
                            ns.diagnostics.push(Diagnostic::error(
                                pt::Loc(
                                    file_no,
                                    single_c.offset,
                                    single_c.offset + single_c.tag.len(),
                                ),
                                "duplicate tag ‘@return’".to_string(),
                            ));
                        } else {
                            res.push(Tag {
                                tag: String::from("return"),
                                no: 0,
                                value: single_c.value.to_owned(),
                            });
                        }
                    } else {
                        let v: Vec<&str> = single_c.value.splitn(2, char::is_whitespace).collect();
                        if v.is_empty() || v[0].is_empty() {
                            ns.diagnostics.push(Diagnostic::error(
                                pt::Loc(
                                    file_no,
                                    single_c.offset,
                                    single_c.offset + single_c.tag.len(),
                                ),
                                "tag ‘@return’ missing parameter name".to_string(),
                            ));
                            continue;
                        }
                        let name = v[0];
                        let value = v.get(1).unwrap_or(&"").to_string();

                        if let Some(no) = returns.iter().position(|p| p.name == name) {
                            if res.iter().any(|e| e.tag == "return" && e.no == no) {
                                ns.diagnostics.push(Diagnostic::error(
                                    pt::Loc(
                                        file_no,
                                        single_c.offset,
                                        single_c.offset + single_c.tag.len(),
                                    ),
                                    format!("duplicate tag ‘@return’ for ‘{}’", name),
                                ));
                            } else {
                                res.push(Tag {
                                    tag: String::from("return"),
                                    no,
                                    value,
                                });
                            }
                        } else {
                            ns.diagnostics.push(Diagnostic::error(
                                pt::Loc(
                                    file_no,
                                    single_c.offset,
                                    single_c.offset + single_c.tag.len(),
                                ),
                                format!("tag ‘@return’ no field ‘{}’", name),
                            ));
                        }
                    }
                }
                "inheritdoc" if bases.is_some() => {
                    if single_c.value.is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(
                                file_no,
                                single_c.offset,
                                single_c.offset + single_c.tag.len(),
                            ),
                            "missing contract for tag ‘@inheritdoc’".to_string(),
                        ));
                    } else if bases.unwrap().iter().any(|s| &single_c.value == s) {
                        res.push(Tag {
                            tag: String::from("inheritdoc"),
                            no: 0,
                            value: single_c.value.to_owned(),
                        });
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc(
                                file_no,
                                single_c.offset,
                                single_c.offset + single_c.tag.len(),
                            ),
                            format!(
                                "base contract ‘{}’ not found in tag ‘@inheritdoc’",
                                single_c.value
                            ),
                        ));
                    }
                }
                _ => {
                    ns.diagnostics.push(Diagnostic::error(
                        pt::Loc(
                            file_no,
                            single_c.offset,
                            single_c.offset + single_c.tag.len(),
                        ),
                        format!("tag ‘@{}’ is not valid for {}", single_c.tag, ty),
                    ));
                }
            }
        }
    }

    res
}

/// Render tags as plain text string
pub fn render(tags: &[Tag]) -> String {
    let mut s = String::new();

    if let Some(tag) = tags.iter().find(|e| e.tag == "title") {
        s.push_str(&tag.value);
        s.push('\n');
        s.push('\n');
    }

    if let Some(tag) = tags.iter().find(|e| e.tag == "notice") {
        s.push_str(&tag.value);
        s.push('\n');
        s.push('\n');
    }

    if let Some(tag) = tags.iter().find(|e| e.tag == "author") {
        s.push_str(&format!("Author: {}", &tag.value));
    }

    s
}
