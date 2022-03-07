use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockParseError {
    #[error("Block must start with BEGIN:")]
    BlockNotStartingWithBEGIN,
}

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

        if let Some(name) = lines[position].strip_prefix("BEGIN:") {
            let mut inner_block_start = None;

            position += 1;
            let mut inner_lines = Vec::new();
            let mut inner_blocks = Vec::new();

            while position < lines.len() {
                let line = &lines[position];
                position += 1;

                if line.starts_with("BEGIN:") {
                    if inner_block_start.is_none() {
                        // only save the first one!
                        inner_block_start = Some(position - 1);
                    }
                    depth += 1;
                } else if line.starts_with("END:") {
                    depth -= 1;

                    if depth == 1 {
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
