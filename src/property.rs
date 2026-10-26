use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLine<'a> {
    pub name: &'a str,
    pub params: Vec<(&'a str, Cow<'a, str>)>,
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
                name_and_params[..semi_idx].trim(),
                Some(&name_and_params[semi_idx + 1..]),
                Some(&line[semi_idx + 1..]),
            ),
            None => (name_and_params.trim(), None, None),
        };

        let mut params = Vec::new();
        if let Some(p_str) = params_str {
            for part in split_outside_quotes(p_str, b';') {
                if let Some((k, v)) = part.split_once('=') {
                    params.push((k, unescape_rfc6868_param(v.trim_matches('"'))));
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

    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_ref())
    }
}

pub fn unescape_rfc6868_param(s: &str) -> Cow<'_, str> {
    if !s.contains('^') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '^' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('^') => out.push('^'),
                Some('\'') => out.push('"'),
                Some(other) => {
                    out.push('^');
                    out.push(other);
                }
                None => out.push('^'),
            }
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
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

#[must_use]
pub fn unescape_ical_text(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 'N') => out.push('\n'),
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

pub fn parse_ical_duration(s: &str) -> Option<chrono::Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
        (-1i64, r)
    } else if let Some(r) = s.strip_prefix('+') {
        (1i64, r)
    } else {
        (1i64, s)
    };

    let rest = rest.strip_prefix('P').or_else(|| rest.strip_prefix('p'))?;
    if rest.is_empty() {
        return None;
    }

    // Check dur-week: <n>W
    if let Some(weeks_str) = rest.strip_suffix('W').or_else(|| rest.strip_suffix('w')) {
        let weeks: i64 = weeks_str.parse().ok()?;
        return Some(chrono::Duration::weeks(sign * weeks));
    }

    let mut total_seconds = 0i64;
    let mut has_any = false;

    let (date_part, time_part) = if let Some((d, t)) = rest.split_once(['T', 't']) {
        if t.is_empty() {
            return None;
        }
        (d, Some(t))
    } else {
        (rest, None)
    };

    if !date_part.is_empty() {
        let days_str = date_part
            .strip_suffix('D')
            .or_else(|| date_part.strip_suffix('d'))?;
        let days: i64 = days_str.parse().ok()?;
        total_seconds += days * 86_400;
        has_any = true;
    }

    if let Some(mut t) = time_part {
        if let Some(idx) = t.find(['H', 'h']) {
            let hours: i64 = t[..idx].parse().ok()?;
            total_seconds += hours * 3_600;
            t = &t[idx + 1..];
            has_any = true;
        }
        if let Some(idx) = t.find(['M', 'm']) {
            let mins: i64 = t[..idx].parse().ok()?;
            total_seconds += mins * 60;
            t = &t[idx + 1..];
            has_any = true;
        }
        if let Some(idx) = t.find(['S', 's']) {
            let secs: i64 = t[..idx].parse().ok()?;
            total_seconds += secs;
            t = &t[idx + 1..];
            has_any = true;
        }
        if !t.is_empty() {
            return None;
        }
    }

    if !has_any {
        return None;
    }

    Some(chrono::Duration::seconds(sign * total_seconds))
}

#[cfg(test)]
#[path = "tests/property.rs"]
mod tests;
