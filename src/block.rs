use crate::errors::BlockParseError;

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
}

impl<'a> TryFrom<&'a [String]> for Block {
    type Error = BlockParseError;

    fn try_from(lines: &'a [String]) -> Result<Self, Self::Error> {
        log::trace!(
            "process_lines_skipping_inner, lines.len() == {}",
            lines.len()
        );

        let mut depth = 1;
        let mut position = 0;

        if let Some(name) = lines.get(position).and_then(|l| l.strip_prefix("BEGIN:")) {
            let mut inner_block_start = None;

            position += 1;
            let mut inner_lines = Vec::new();
            let mut inner_blocks = Vec::new();

            while position < lines.len() {
                let line = &lines[position];
                position += 1;

                if depth == 0 {
                    if line.trim().is_empty() {
                        continue;
                    } else {
                        return Err(BlockParseError::UnexpectedContentAfterEnd);
                    }
                }

                if line.starts_with("BEGIN:") {
                    if inner_block_start.is_none() {
                        // only save the first one!
                        inner_block_start = Some(position - 1);
                    }
                    depth += 1;
                } else if let Some(end_name) = line.strip_prefix("END:") {
                    depth -= 1;

                    if depth == 0 {
                        if end_name != name {
                            return Err(BlockParseError::MismatchedEndTag {
                                expected: name.to_owned(),
                                found: end_name.to_owned(),
                            });
                        }
                    } else if depth == 1 {
                        // process inner!
                        log::trace!(
                            "About to go in {}..{}",
                            inner_block_start.unwrap(),
                            position
                        );
                        inner_blocks.push(lines[inner_block_start.unwrap()..position].try_into()?);
                        inner_block_start = None;
                    }
                } else if depth == 1 {
                    inner_lines.push(line.to_owned());
                }
            }

            if depth != 0 {
                return Err(BlockParseError::UnclosedBlock {
                    name: name.to_owned(),
                });
            }

            Ok(Block {
                name: name.to_owned(),
                inner_lines,
                inner_blocks,
            })
        } else {
            Err(BlockParseError::BlockNotStartingWithBEGIN)
        }
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
}
