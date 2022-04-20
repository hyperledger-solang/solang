// Parse the fields f
use crate::lexer::CommentType;
use crate::pt::{DocComment, SingleDocComment};

/// Convert the comment to lines, stripping
fn to_lines<'a>(
    comments: &[(usize, CommentType, &'a str)],
) -> Vec<(CommentType, Vec<(usize, &'a str)>)> {
    let mut res = Vec::new();

    for (start, ty, comment) in comments.iter() {
        let mut grouped_comments = Vec::new();

        match ty {
            CommentType::Line => {
                let leading = comment.chars().take_while(|ch| ch.is_whitespace()).count();

                grouped_comments.push((*start + leading, comment.trim()));
            }
            CommentType::Block => {
                let mut start = *start;

                for s in comment.lines() {
                    if let Some((i, _)) = s
                        .char_indices()
                        .find(|(_, ch)| !ch.is_whitespace() && *ch != '*')
                    {
                        grouped_comments.push((start + i, s[i..].trim_end()));
                    }

                    start += s.len() + 1;
                }
            }
        }

        res.push((*ty, grouped_comments));
    }

    res
}

// Parse the DocComments tags
pub fn tags(lines: &[(usize, CommentType, &str)]) -> Vec<DocComment> {
    // first extract the tags
    let mut tags = Vec::new();

    let lines = to_lines(lines);
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
                single_tags.push(SingleDocComment {
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

                single_tags.push(SingleDocComment {
                    tag_offset: start_offset + start_offset + leading,
                    tag: String::from("notice"),
                    value_offset: start_offset + start_offset + leading,
                    value: line.trim().to_owned(),
                });
            }
        }

        match ty {
            CommentType::Line if !single_tags.is_empty() => tags.push(DocComment::Line {
                comment: single_tags[0].to_owned(),
            }),
            CommentType::Block => tags.push(DocComment::Block {
                comments: single_tags,
            }),
            _ => {}
        }
    }

    tags
}
