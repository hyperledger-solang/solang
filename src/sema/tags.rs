use super::ast::{Diagnostic, Namespace, Parameter, Tag};
use solang_parser::pt;
use std::fmt::Write;

#[derive(Debug, PartialEq, Clone)]
pub enum DocComment {
    Line { comment: DocCommentTag },
    Block { comments: Vec<DocCommentTag> },
}

impl DocComment {
    pub fn comments(&self) -> Vec<&DocCommentTag> {
        match self {
            DocComment::Line { comment } => vec![comment],
            DocComment::Block { comments } => comments.iter().collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocCommentTag {
    pub tag: String,
    pub tag_offset: usize,
    pub value: String,
    pub value_offset: usize,
}

// Parse the DocComments tags from the parse tree
pub fn parse_doccomments(comments: &[pt::Comment], start: usize, end: usize) -> Vec<DocComment> {
    // first extract the tags
    let mut tags = Vec::new();

    let lines = to_lines(comments, start, end);

    for (ty, comment_lines) in lines {
        let mut single_tags = Vec::new();

        for (start_offset, line) in comment_lines {
            let mut chars = line.char_indices().peekable();

            if let Some((_, '@')) = chars.peek() {
                // step over @
                let (tag_start, _) = chars.next().unwrap();
                let mut tag_end = tag_start;

                while let Some((offset, c)) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }

                    tag_end = *offset;

                    chars.next();
                }

                let leading = line[tag_end + 1..]
                    .chars()
                    .take_while(|ch| ch.is_whitespace())
                    .count();

                // tag value
                single_tags.push(DocCommentTag {
                    tag_offset: start_offset + tag_start + 1,
                    tag: line[tag_start + 1..tag_end + 1].to_owned(),
                    value_offset: start_offset + tag_end + leading + 1,
                    value: line[tag_end + 1..].trim().to_owned(),
                });
            } else if !single_tags.is_empty() || !tags.is_empty() {
                let line = line.trim();
                if !line.is_empty() {
                    let single_doc_comment = if let Some(single_tag) = single_tags.last_mut() {
                        Some(single_tag)
                    } else if let Some(tag) = tags.last_mut() {
                        match tag {
                            DocComment::Line { comment } => Some(comment),
                            DocComment::Block { comments } => comments.last_mut(),
                        }
                    } else {
                        None
                    };

                    if let Some(comment) = single_doc_comment {
                        comment.value.push('\n');
                        comment.value.push_str(line);
                    }
                }
            } else {
                let leading = line.chars().take_while(|ch| ch.is_whitespace()).count();

                single_tags.push(DocCommentTag {
                    tag_offset: start_offset + start_offset + leading,
                    tag: String::from("notice"),
                    value_offset: start_offset + start_offset + leading,
                    value: line.trim().to_owned(),
                });
            }
        }

        match ty {
            pt::CommentType::Line if !single_tags.is_empty() => tags.push(DocComment::Line {
                comment: single_tags[0].to_owned(),
            }),
            pt::CommentType::Block => tags.push(DocComment::Block {
                comments: single_tags,
            }),
            _ => {}
        }
    }

    tags
}

/// Convert the comment to lines, stripping whitespace and leading * in block comments
fn to_lines(
    comments: &[pt::Comment],
    start: usize,
    end: usize,
) -> Vec<(pt::CommentType, Vec<(usize, &str)>)> {
    let mut res = Vec::new();

    for comment in comments.iter() {
        let mut grouped_comments = Vec::new();

        match comment {
            pt::Comment::DocLine(loc, comment) => {
                if loc.start() >= end || loc.end() < start {
                    continue;
                }

                // remove the leading ///
                let leading = comment[3..]
                    .chars()
                    .take_while(|ch| ch.is_whitespace())
                    .count();

                grouped_comments.push((loc.start() + leading + 3, comment[3..].trim()));

                res.push((pt::CommentType::Line, grouped_comments));
            }
            pt::Comment::DocBlock(loc, comment) => {
                if loc.start() >= end || loc.end() < start {
                    continue;
                }

                let mut start = loc.start() + 3;

                let len = comment.len();

                // remove the leading /** and tailing */
                for s in comment[3..len - 2].lines() {
                    if let Some((i, _)) = s
                        .char_indices()
                        .find(|(_, ch)| !ch.is_whitespace() && *ch != '*')
                    {
                        grouped_comments.push((start + i, s[i..].trim_end()));
                    }

                    start += s.len() + 1;
                }

                res.push((pt::CommentType::Block, grouped_comments));
            }
            _ => (),
        }
    }

    res
}

/// Resolve the tags for a type from parsed doccomment
pub fn resolve_tags(
    file_no: usize,
    ty: &str,
    tags: &[DocComment],
    params: Option<&[Parameter]>,
    returns: Option<&[Parameter]>,
    bases: Option<&[String]>,
    ns: &mut Namespace,
) -> Vec<Tag> {
    let mut res: Vec<Tag> = Vec::new();

    for c in tags.iter().flat_map(DocComment::comments) {
        match c.tag.as_str() {
            "notice" | "author" | "title" | "dev" => {
                // fold fields with the same name
                if let Some(prev) = res.iter_mut().find(|e| e.tag == c.tag) {
                    prev.value.push(' ');
                    prev.value.push_str(&c.value);
                } else {
                    res.push(Tag {
                        tag: c.tag.to_owned(),
                        value: c.value.to_owned(),
                        no: 0,
                    })
                }
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
                    if res.iter().any(|e| e.tag == "param" && e.no == no) {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                            format!("duplicate tag '@param' for '{}'", name),
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
                        pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len()),
                        format!("tag '@param' no field '{}'", name),
                    ));
                }
            }
            "return" if returns.is_some() => {
                let returns = returns.unwrap();

                if returns.is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                        "tag '@return' for function with no return values".to_string(),
                    ));
                } else if returns.len() == 1 {
                    if res.iter().any(|e| e.tag == "return") {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                            "duplicate tag '@return'".to_string(),
                        ));
                    } else {
                        res.push(Tag {
                            tag: String::from("return"),
                            no: 0,
                            value: c.value.to_owned(),
                        });
                    }
                } else {
                    let v: Vec<&str> = c.value.splitn(2, char::is_whitespace).collect();
                    if v.is_empty() || v[0].is_empty() {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len()),
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
                                pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                                format!("duplicate tag '@return' for '{}'", name),
                            ));
                        } else {
                            res.push(Tag {
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
                            tag: String::from("return"),
                            no,
                            value,
                        });
                    } else {
                        ns.diagnostics.push(Diagnostic::error(
                            pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len()),
                            format!("tag '@return' no matching return value '{}'", c.value),
                        ));
                    }
                }
            }
            "inheritdoc" if bases.is_some() => {
                if c.value.is_empty() {
                    ns.diagnostics.push(Diagnostic::error(
                        pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                        "missing contract for tag '@inheritdoc'".to_string(),
                    ));
                } else if bases.unwrap().iter().any(|s| &c.value == s) {
                    res.push(Tag {
                        tag: String::from("inheritdoc"),
                        no: 0,
                        value: c.value.to_owned(),
                    });
                } else {
                    ns.diagnostics.push(Diagnostic::error(
                        pt::Loc::File(file_no, c.value_offset, c.value_offset + c.value.len()),
                        format!("base contract '{}' not found in tag '@inheritdoc'", c.value),
                    ));
                }
            }
            _ => {
                ns.diagnostics.push(Diagnostic::error(
                    pt::Loc::File(file_no, c.tag_offset, c.tag_offset + c.tag.len()),
                    format!("tag '@{}' is not valid for {}", c.tag, ty),
                ));
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
        write!(s, "Author: {}", &tag.value).unwrap();
    }

    s
}
