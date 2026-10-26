use std::{borrow::Cow, iter::Peekable, str::Lines};

#[derive(Debug, Clone)]
pub struct ICalLineParser<'a> {
    lines: Peekable<Lines<'a>>,
}

impl<'a> ICalLineParser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lines: text.lines().peekable(),
        }
    }
}

impl<'a> Iterator for ICalLineParser<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.lines.next()?;

        match self
            .lines
            .peek()
            .and_then(|next| next.strip_prefix(' ').or_else(|| next.strip_prefix('\t')))
        {
            None => Some(Cow::Borrowed(first)),
            Some(_) => {
                let mut owned = String::from(first);
                while let Some(stripped) = self
                    .lines
                    .peek()
                    .and_then(|next| next.strip_prefix(' ').or_else(|| next.strip_prefix('\t')))
                {
                    owned.push_str(stripped);
                    self.lines.next();
                }
                Some(Cow::Owned(owned))
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/ical_line_parser.rs"]
mod tests;
