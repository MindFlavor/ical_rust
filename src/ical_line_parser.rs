#[derive(Debug, Clone)]
pub struct ICalLineParser<'a> {
    pub lines: &'a [&'a str],
    pub position: usize,
}

impl<'a> ICalLineParser<'a> {
    pub fn new(lines: &'a [&'a str]) -> Self {
        Self { lines, position: 0 }
    }
}

impl<'a> Iterator for ICalLineParser<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut s = None;
        let mut count = 0;

        while self.position + count < self.lines.len() {
            let line = self.lines[self.position + count];

            if count == 0 {
                s = Some(line.to_owned());
                count += 1;
            } else if let Some(stripped) = line.strip_prefix(' ') {
                s = Some(s.unwrap_or_default() + stripped);
                count += 1;
            } else {
                break;
            }
        }

        self.position += count;

        s
    }
}
