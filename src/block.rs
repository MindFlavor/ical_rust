use crate::{errors::BlockParseError, ical_line_parser::ICalLineParser};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    pub inner_lines: Vec<String>,
    pub inner_blocks: Vec<Block>,
}

impl Block {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn from_lines<I, S>(lines: I) -> Result<Self, BlockParseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String> + AsRef<str>,
    {
        let mut stack: Vec<Block> = Vec::new();
        let mut root: Option<Block> = None;

        for line in lines {
            let s = line.as_ref();

            if root.is_some() {
                if s.trim().is_empty() {
                    continue;
                } else {
                    return Err(BlockParseError::UnexpectedContentAfterEnd);
                }
            }

            if s.len() >= 6 && s.is_char_boundary(6) && s[..6].eq_ignore_ascii_case("BEGIN:") {
                stack.push(Block {
                    name: s[6..].to_ascii_uppercase(),
                    inner_lines: Vec::new(),
                    inner_blocks: Vec::new(),
                });
            } else if s.len() >= 4 && s.is_char_boundary(4) && s[..4].eq_ignore_ascii_case("END:") {
                let end_name = s[4..].to_ascii_uppercase();
                let top = stack
                    .pop()
                    .ok_or(BlockParseError::BlockNotStartingWithBEGIN)?;
                if top.name != end_name {
                    return Err(BlockParseError::MismatchedEndTag {
                        expected: top.name,
                        found: end_name,
                    });
                }
                if let Some(parent) = stack.last_mut() {
                    parent.inner_blocks.push(top);
                } else {
                    root = Some(top);
                }
            } else if let Some(current) = stack.last_mut() {
                current.inner_lines.push(line.into());
            } else if !s.trim().is_empty() {
                return Err(BlockParseError::BlockNotStartingWithBEGIN);
            }
        }

        if let Some(unclosed) = stack.pop() {
            return Err(BlockParseError::UnclosedBlock {
                name: unclosed.name,
            });
        }

        root.ok_or(BlockParseError::BlockNotStartingWithBEGIN)
    }
}

impl TryFrom<&str> for Block {
    type Error = BlockParseError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::from_lines(ICalLineParser::new(text))
    }
}

impl FromStr for Block {
    type Err = BlockParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl<'a> TryFrom<&'a [String]> for Block {
    type Error = BlockParseError;

    fn try_from(lines: &'a [String]) -> Result<Self, Self::Error> {
        Self::from_lines(lines.iter().map(String::as_str))
    }
}

#[cfg(test)]
#[path = "tests/block.rs"]
mod tests;
