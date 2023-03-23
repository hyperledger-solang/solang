// SPDX-License-Identifier: Apache-2.0

//! Solidity parsed doc comments.
//!
//! See also the Solidity documentation on [natspec].
//!
//! [natspec]: https://docs.soliditylang.org/en/latest/natspec-format.html

use crate::pt::Comment;

/// A Solidity parsed doc comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocComment {
    /// A line doc comment.
    ///
    /// `/// doc comment`
    Line {
        /// The single comment tag of the line.
        comment: DocCommentTag,
    },

    /// A block doc comment.
    ///
    /// ```text
    /// /**
    ///  * block doc comment
    ///  */
    /// ```
    Block {
        /// The list of doc comment tags of the block.
        comments: Vec<DocCommentTag>,
    },
}

impl DocComment {
    /// Returns the inner comments.
    pub fn comments(&self) -> Vec<&DocCommentTag> {
        match self {
            DocComment::Line { comment } => vec![comment],
            DocComment::Block { comments } => comments.iter().collect(),
        }
    }

    /// Consumes `self` to return the inner comments.
    pub fn into_comments(self) -> Vec<DocCommentTag> {
        match self {
            DocComment::Line { comment } => vec![comment],
            DocComment::Block { comments } => comments,
        }
    }
}

/// A Solidity doc comment's tag, value and respective offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocCommentTag {
    /// The tag of the doc comment, like the `notice` in `/// @notice Doc comment value`
    pub tag: String,
    /// The offset of the comment's tag, relative to the start of the source string.
    pub tag_offset: usize,
    /// The actual comment string, like `Doc comment value` in `/// @notice Doc comment value`
    pub value: String,
    /// The offset of the comment's value, relative to the start of the source string.
    pub value_offset: usize,
}

enum CommentType {
    Line,
    Block,
}

/// From the start to end offset, filter all the doc comments out of the comments and parse
/// them into tags with values.
pub fn parse_doccomments(comments: &[Comment], start: usize, end: usize) -> Vec<DocComment> {
    let mut tags = Vec::with_capacity(comments.len());

    for (ty, comment_lines) in filter_comments(comments, start, end) {
        let mut single_tags = Vec::with_capacity(comment_lines.len());

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
            CommentType::Line if !single_tags.is_empty() => tags.push(DocComment::Line {
                comment: single_tags.swap_remove(0),
            }),
            CommentType::Block => tags.push(DocComment::Block {
                comments: single_tags,
            }),
            _ => {}
        }
    }

    tags
}

/// Convert the comment to lines, stripping whitespace, comment characters and leading * in block comments
fn filter_comments(
    comments: &[Comment],
    start: usize,
    end: usize,
) -> impl Iterator<Item = (CommentType, Vec<(usize, &str)>)> {
    comments.iter().filter_map(move |comment| {
        match comment {
            // filter out all non-doc comments
            Comment::Block(..) | Comment::Line(..) => None,
            // filter out doc comments that are outside the given range
            Comment::DocLine(loc, _) | Comment::DocBlock(loc, _)
                if loc.start() >= end || loc.end() < start =>
            {
                None
            }

            Comment::DocLine(loc, comment) => {
                // remove the leading /// and whitespace;
                // if we don't find a match, default to skipping the 3 `/` bytes,
                // since they are guaranteed to be in the comment string
                let leading = comment
                    .find(|c: char| c != '/' && !c.is_whitespace())
                    .unwrap_or(3);
                let comment = (loc.start() + leading, comment[leading..].trim_end());
                Some((CommentType::Line, vec![comment]))
            }
            Comment::DocBlock(loc, comment) => {
                // remove the leading /** and tailing */
                let mut start = loc.start() + 3;
                let mut grouped_comments = Vec::new();
                let len = comment.len();
                for s in comment[3..len - 2].lines() {
                    if let Some((i, _)) = s
                        .char_indices()
                        .find(|(_, ch)| !ch.is_whitespace() && *ch != '*')
                    {
                        grouped_comments.push((start + i, s[i..].trim_end()));
                    }

                    start += s.len() + 1;
                }
                Some((CommentType::Block, grouped_comments))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let src = r#"
pragma solidity ^0.8.19;
/// @name Test
///  no tag
///@notice    Cool contract    
///   @  dev     This is not a dev tag 
/**
 * @dev line one
 *    line 2
 */
contract Test {
    /*** my function    
          i like whitespace    
*/
    function test() {}
}
"#;
        let (_, comments) = crate::parse(src, 0).unwrap();
        assert_eq!(comments.len(), 6);

        let actual = parse_doccomments(&comments, 0, usize::MAX);
        let expected = vec![
            DocComment::Line {
                comment: DocCommentTag {
                    tag: "name".into(),
                    tag_offset: 31,
                    value: "Test\nno tag".into(),
                    value_offset: 36,
                },
            },
            DocComment::Line {
                comment: DocCommentTag {
                    tag: "notice".into(),
                    tag_offset: 57,
                    value: "Cool contract".into(),
                    value_offset: 67,
                },
            },
            DocComment::Line {
                comment: DocCommentTag {
                    tag: "".into(),
                    tag_offset: 92,
                    value: "dev     This is not a dev tag".into(),
                    value_offset: 94,
                },
            },
            // TODO: Second block is merged into the first
            DocComment::Block {
                comments: vec![DocCommentTag {
                    tag: "dev".into(),
                    tag_offset: 133,
                    value: "line one\nline 2\nmy function\ni like whitespace".into(),
                    value_offset: 137,
                }],
            },
            DocComment::Block { comments: vec![] },
        ];

        assert_eq!(actual, expected);
    }
}
