#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLine<'a> {
    pub name: &'a str,
    pub params: Vec<(&'a str, &'a str)>,
    pub value: &'a str,
    pub raw_after_semicolon: Option<&'a str>,
}

impl<'a> ContentLine<'a> {
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut in_quotes = false;
        let mut colon_idx = None;
        for (idx, b) in line.bytes().enumerate() {
            if b == b'"' {
                in_quotes = !in_quotes;
            } else if b == b':' && !in_quotes {
                colon_idx = Some(idx);
                break;
            }
        }

        let colon_idx = colon_idx?;
        let name_and_params = &line[..colon_idx];
        let value = &line[colon_idx + 1..];

        let (name, params_str, raw_after_semicolon) = match name_and_params.find(';') {
            Some(semi_idx) => (
                &name_and_params[..semi_idx],
                Some(&name_and_params[semi_idx + 1..]),
                Some(&line[semi_idx + 1..]),
            ),
            None => (name_and_params, None, None),
        };

        let mut params = Vec::new();
        if let Some(p_str) = params_str {
            for part in split_outside_quotes(p_str, b';') {
                if let Some((k, v)) = part.split_once('=') {
                    params.push((k, v.trim_matches('"')));
                }
            }
        }

        Some(Self {
            name,
            params,
            value,
            raw_after_semicolon,
        })
    }

    pub fn param(&self, key: &str) -> Option<&'a str> {
        self.params
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| *v)
    }
}

pub(crate) fn split_outside_quotes(s: &str, delimiter: u8) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    for (idx, b) in s.bytes().enumerate() {
        if b == b'"' {
            in_quotes = !in_quotes;
        } else if b == delimiter && !in_quotes {
            result.push(&s[start..idx]);
            start = idx + 1;
        }
    }
    result.push(&s[start..]);
    result
}

pub fn unescape_ical_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(';') => out.push(';'),
                Some(',') => out.push(','),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_content_line_with_multiple_and_quoted_params() {
        let line =
            ContentLine::parse("DTSTART;VALUE=DATE-TIME;TZID=\"Europe/Rome\":20230101T090000")
                .unwrap();
        assert_eq!(line.name, "DTSTART");
        assert_eq!(line.param("VALUE"), Some("DATE-TIME"));
        assert_eq!(line.param("TZID"), Some("Europe/Rome"));
        assert_eq!(line.value, "20230101T090000");
    }

    #[test]
    fn unescape_rfc5545_text() {
        let raw = r"Line 1\nLine 2\NComma\, Semicolon\; Backslash\\";
        assert_eq!(
            unescape_ical_text(raw),
            "Line 1\nLine 2\nComma, Semicolon; Backslash\\"
        );
    }
}
