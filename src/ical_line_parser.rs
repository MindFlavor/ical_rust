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
mod tests {
    use super::*;

    #[test]
    fn unfolded_lines_are_borrowed_and_folded_lines_are_owned() {
        let input = "BEGIN:VCALENDAR\r\nSUMMARY:Hello \r\n World\r\n\t!\r\nEND:VCALENDAR";
        let parsed: Vec<Cow<'_, str>> = ICalLineParser::new(input).collect();
        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], Cow::Borrowed("BEGIN:VCALENDAR")));
        assert!(matches!(parsed[1], Cow::Owned(_)));
        assert_eq!(parsed[1], "SUMMARY:Hello World!");
        assert!(matches!(parsed[2], Cow::Borrowed("END:VCALENDAR")));
    }
}
