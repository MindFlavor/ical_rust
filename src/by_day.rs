use crate::errors::ByDayParseError;
use chrono::Weekday;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeekdayRule {
    pub offset: Option<i8>,
    pub weekday: Weekday,
}

impl WeekdayRule {
    #[must_use]
    pub fn new(offset: Option<i8>, weekday: Weekday) -> Self {
        Self { offset, weekday }
    }

    #[must_use]
    pub fn plain(weekday: Weekday) -> Self {
        Self {
            offset: None,
            weekday,
        }
    }

    #[must_use]
    pub fn with_offset(offset: i8, weekday: Weekday) -> Self {
        Self {
            offset: Some(offset),
            weekday,
        }
    }
}

impl fmt::Display for WeekdayRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "{offset}")?;
        }
        write!(f, "{}", weekday_to_ical_str(self.weekday))
    }
}

pub(crate) fn weekday_to_ical_str(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Sun => "SU",
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
    }
}

impl FromStr for WeekdayRule {
    type Err = ByDayParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() || s.len() < 2 {
            return Err(ByDayParseError::InvalidWeekday { w: s.to_owned() });
        }

        if s.len() == 2 {
            let weekday = to_chrono_weekday(s)?;
            Ok(Self::plain(weekday))
        } else {
            let weekday = to_chrono_weekday(&s[s.len() - 2..])?;
            let offset: i8 = s[..s.len() - 2].parse()?;
            if offset == 0 {
                return Err(ByDayParseError::ZeroDelta);
            }
            Ok(Self::with_offset(offset, weekday))
        }
    }
}

pub fn parse_by_day(s: &str) -> Result<Vec<WeekdayRule>, ByDayParseError> {
    let tokens: Vec<_> = s.split(',').filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return Err(ByDayParseError::InvalidWeekday { w: s.to_owned() });
    }
    tokens.into_iter().map(WeekdayRule::from_str).collect()
}

pub(crate) fn to_chrono_weekday(s: &str) -> Result<Weekday, ByDayParseError> {
    match s.as_bytes() {
        [b1, b2] => match [b1.to_ascii_uppercase(), b2.to_ascii_uppercase()] {
            [b'S', b'U'] => Ok(Weekday::Sun),
            [b'M', b'O'] => Ok(Weekday::Mon),
            [b'T', b'U'] => Ok(Weekday::Tue),
            [b'W', b'E'] => Ok(Weekday::Wed),
            [b'T', b'H'] => Ok(Weekday::Thu),
            [b'F', b'R'] => Ok(Weekday::Fri),
            [b'S', b'A'] => Ok(Weekday::Sat),
            _ => Err(ByDayParseError::InvalidWeekday { w: s.to_owned() }),
        },
        _ => Err(ByDayParseError::InvalidWeekday { w: s.to_owned() }),
    }
}

#[cfg(test)]
#[path = "tests/by_day.rs"]
mod tests;
