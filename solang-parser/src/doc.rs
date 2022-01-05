// Parse the fields f
use crate::lexer::CommentType;
use crate::pt::{DocComment, SingleDocComment};

/// Convert the comment to lines, stripping
fn to_lines<'a>(
    comments: &[(usize, CommentType, &'a str)],
) -> Vec<(CommentType, Vec<(usize, &'a str)>)> {
    let mut res = Vec::new();

    for (start, ty, comment) in comments.iter() {
        match ty {
            CommentType::Line => res.push((*ty, vec![(*start, comment.trim())])),
            CommentType::Block => {
                let mut start = *start;
                let mut block_comments = Vec::new();

                for s in comment.lines() {
                    if let Some((i, _)) = s
                        .char_indices()
                        .find(|(_, ch)| !ch.is_whitespace() && *ch != '*')
                    {
                        block_comments.push((start + i, s[i..].trim_end()));
                    }

                    start += s.len();
                }

                res.push((*ty, block_comments));
            }
        }
    }

    res
}

// Parse the DocComments tags
pub fn tags(lines: &[(usize, CommentType, &str)]) -> Vec<DocComment> {
    // first extract the tags
    let mut tags = Vec::new();

    let lines = to_lines(lines);
    for (ty, comment_lines) in lines {
        let mut tags_buffer = Vec::new();

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

                // tag value
                tags_buffer.push(SingleDocComment {
                    offset: tag_start,
                    tag: line[tag_start + 1..tag_end + 1].to_owned(),
                    value: line[tag_end + 1..].trim().to_owned(),
                });
            } else if let Some(tag) = tags_buffer.last_mut() {
                let line = line.trim();
                if !line.is_empty() {
                    tag.value.push(' ');
                    tag.value.push_str(line.trim());
                }
            } else if let Some(tag) = tags.last_mut() {
                let line = line.trim();
                if !line.is_empty() {
                    match tag {
                        DocComment::Line { comment } => {
                            comment.value.push(' ');
                            comment.value.push_str(line.trim());
                        }
                        DocComment::Block { comments } => {
                            comments[0].value.push(' ');
                            comments[0].value.push_str(line.trim());
                        }
                    }
                }
            } else {
                tags_buffer.push(SingleDocComment {
                    offset: start_offset,
                    tag: String::from("notice"),
                    value: line.trim().to_owned(),
                });
            }
        }

        match ty {
            CommentType::Line if !tags_buffer.is_empty() => tags.push(DocComment::Line {
                comment: tags_buffer[0].to_owned(),
            }),
            CommentType::Block => tags.push(DocComment::Block {
                comments: tags_buffer,
            }),
            _ => {}
        }
    }

    tags
}
