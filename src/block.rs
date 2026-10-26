use crate::{errors::BlockParseError, ical_line_parser::ICalLineParser};

#[derive(Debug, Clone, Default)]
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

            if let Some(begin_name) = s.strip_prefix("BEGIN:") {
                stack.push(Block {
                    name: begin_name.to_owned(),
                    inner_lines: Vec::new(),
                    inner_blocks: Vec::new(),
                });
            } else if let Some(end_name) = s.strip_prefix("END:") {
                let top = stack
                    .pop()
                    .ok_or(BlockParseError::BlockNotStartingWithBEGIN)?;
                if top.name != end_name {
                    return Err(BlockParseError::MismatchedEndTag {
                        expected: top.name,
                        found: end_name.to_owned(),
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

impl<'a> TryFrom<&'a [String]> for Block {
    type Error = BlockParseError;

    fn try_from(lines: &'a [String]) -> Result<Self, Self::Error> {
        Self::from_lines(lines.iter().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue1_empty_input_does_not_panic() {
        let empty: &[String] = &[];
        let res = Block::try_from(empty);
        assert!(matches!(
            res,
            Err(BlockParseError::BlockNotStartingWithBEGIN)
        ));
    }

    #[test]
    fn test_issue2_unclosed_and_mismatched_blocks_fail() {
        let unclosed = vec!["BEGIN:VCALENDAR".to_string(), "BEGIN:VEVENT".to_string()];
        assert!(Block::try_from(unclosed.as_slice()).is_err());

        let mismatched = vec!["BEGIN:VCALENDAR".to_string(), "END:VEVENT".to_string()];
        assert!(Block::try_from(mismatched.as_slice()).is_err());

        let extra_end = vec![
            "BEGIN:VCALENDAR".to_string(),
            "END:VCALENDAR".to_string(),
            "END:VCALENDAR".to_string(),
        ];
        assert!(Block::try_from(extra_end.as_slice()).is_err());
    }

    #[test]
    fn test_improvement3_single_pass_nested_blocks_from_str() {
        let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VTIMEZONE\r\n\
TZID:Europe/Rome\r\n\
BEGIN:STANDARD\r\n\
TZNAME:CET\r\n\
END:STANDARD\r\n\
END:VTIMEZONE\r\n\
END:VCALENDAR\r\n";

        let block = Block::try_from(input).unwrap();
        assert_eq!(block.name, "VCALENDAR");
        assert_eq!(block.inner_lines, vec!["VERSION:2.0"]);
        assert_eq!(block.inner_blocks.len(), 1);
        assert_eq!(block.inner_blocks[0].name, "VTIMEZONE");
        assert_eq!(block.inner_blocks[0].inner_blocks.len(), 1);
        assert_eq!(block.inner_blocks[0].inner_blocks[0].name, "STANDARD");
    }
}
