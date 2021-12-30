// Parse the fields f
use crate::lexer::CommentType;
use crate::pt::DocComment;

/// Convert the comment to lines, stripping
fn to_lines<'a>(comments: &[(usize, CommentType, &'a str)]) -> Vec<(usize, &'a str)> {
    let mut res = Vec::new();

    for (start, ty, comment) in comments.iter() {
        match ty {
            CommentType::Line => res.push((*start, comment.trim())),
            CommentType::Block => {
                let mut start = *start;

                for s in comment.lines() {
                    if let Some((i, _)) = s
                        .char_indices()
                        .find(|(_, ch)| !ch.is_whitespace() && *ch != '*')
                    {
                        res.push((start + i, s[i..].trim_end()))
                    }

                    start += s.len();
                }
            }
        }
    }

    res
}

// Parse the DocComments tags
pub fn tags(lines: &[(usize, CommentType, &str)]) -> Vec<DocComment> {
    // first extract the tags
    let mut tags = Vec::new();

    for (start_offset, line) in to_lines(lines).into_iter() {
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
            tags.push(DocComment {
                offset: tag_start,
                tag: line[tag_start + 1..tag_end + 1].to_owned(),
                value: line[tag_end + 1..].trim().to_owned(),
            });
        } else if let Some(tag) = tags.last_mut() {
            let line = line.trim();
            if !line.is_empty() {
                tag.value.push(' ');
                tag.value.push_str(line.trim());
            }
        } else {
            tags.push(DocComment {
                offset: start_offset,
                tag: String::from("notice"),
                value: line.trim().to_owned(),
            });
        }
    }

    tags
}
