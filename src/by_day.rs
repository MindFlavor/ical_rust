use crate::errors::ByDayParseError;
use chrono::Weekday;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeekdayRule {
    pub offset: Option<i8>,
    pub weekday: Weekday,
}

impl WeekdayRule {
    pub fn new(offset: Option<i8>, weekday: Weekday) -> Self {
        Self { offset, weekday }
    }

    pub fn plain(weekday: Weekday) -> Self {
        Self {
            offset: None,
            weekday,
        }
    }

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
    match s.to_ascii_uppercase().as_str() {
        "SU" => Ok(Weekday::Sun),
        "MO" => Ok(Weekday::Mon),
        "TU" => Ok(Weekday::Tue),
        "WE" => Ok(Weekday::Wed),
        "TH" => Ok(Weekday::Thu),
        "FR" => Ok(Weekday::Fri),
        "SA" => Ok(Weekday::Sat),
        _ => Err(ByDayParseError::InvalidWeekday { w: s.to_owned() }),
    }
}

#[cfg(test)]
#[path = "tests/by_day.rs"]
mod tests;
