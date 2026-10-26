use crate::{errors::BlockParseError, ical_line_parser::ICalLineParser};
use std::{borrow::Cow, str::FromStr};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Block<'a> {
    pub name: String,
    pub inner_lines: Vec<Cow<'a, str>>,
    pub inner_blocks: Vec<Block<'a>>,
}

impl<'a> Block<'a> {
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn from_lines<I, S>(lines: I) -> Result<Self, BlockParseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'a, str>>,
    {
        let mut stack: Vec<Block<'a>> = Vec::new();
        let mut root: Option<Block<'a>> = None;

        for line in lines {
            let line: Cow<'a, str> = line.into();
            let s = line.as_ref();

            if root.is_some() {
                if s.trim().is_empty() {
                    continue;
                }
                return Err(BlockParseError::UnexpectedContentAfterEnd);
            }

            if let Some(rest) = s
                .get(..6)
                .filter(|p| p.eq_ignore_ascii_case("BEGIN:"))
                .and_then(|_| s.get(6..))
            {
                stack.push(Block {
                    name: rest.to_ascii_uppercase(),
                    inner_lines: Vec::new(),
                    inner_blocks: Vec::new(),
                });
            } else if let Some(rest) = s
                .get(..4)
                .filter(|p| p.eq_ignore_ascii_case("END:"))
                .and_then(|_| s.get(4..))
            {
                let top = stack
                    .pop()
                    .ok_or(BlockParseError::BlockNotStartingWithBEGIN)?;
                if !top.name.eq_ignore_ascii_case(rest) {
                    return Err(BlockParseError::MismatchedEndTag {
                        expected: top.name,
                        found: rest.to_ascii_uppercase(),
                    });
                }
                if let Some(parent) = stack.last_mut() {
                    parent.inner_blocks.push(top);
                } else {
                    root = Some(top);
                }
            } else if let Some(current) = stack.last_mut() {
                current.inner_lines.push(line);
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

impl<'a> TryFrom<&'a str> for Block<'a> {
    type Error = BlockParseError;

    fn try_from(text: &'a str) -> Result<Self, Self::Error> {
        Self::from_lines(ICalLineParser::new(text))
    }
}

impl FromStr for Block<'static> {
    type Err = BlockParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_lines(ICalLineParser::new(s).map(|c| Cow::Owned(c.into_owned())))
    }
}

impl<'a> TryFrom<&'a [String]> for Block<'a> {
    type Error = BlockParseError;

    fn try_from(lines: &'a [String]) -> Result<Self, Self::Error> {
        Self::from_lines(lines.iter().map(String::as_str))
    }
}

#[cfg(test)]
#[path = "tests/block.rs"]
mod tests;
