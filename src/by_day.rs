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
mod tests {
    use super::*;

    #[test]
    fn parse_00() {
        let s = "MO,TU,FR";
        let rules = parse_by_day(s).unwrap();
        assert_eq!(
            rules,
            vec![
                WeekdayRule::plain(Weekday::Mon),
                WeekdayRule::plain(Weekday::Tue),
                WeekdayRule::plain(Weekday::Fri),
            ]
        );
    }

    #[test]
    fn parse_delta() {
        let r1: WeekdayRule = "-20MO".parse().unwrap();
        assert_eq!(r1, WeekdayRule::with_offset(-20, Weekday::Mon));
        let r2: WeekdayRule = "30FR".parse().unwrap();
        assert_eq!(r2, WeekdayRule::with_offset(30, Weekday::Fri));
    }

    #[test]
    fn parse_multi_delta() {
        let rules = parse_by_day("1MO,3MO,-1FR").unwrap();
        assert_eq!(
            rules,
            vec![
                WeekdayRule::with_offset(1, Weekday::Mon),
                WeekdayRule::with_offset(3, Weekday::Mon),
                WeekdayRule::with_offset(-1, Weekday::Fri),
            ]
        );
    }

    #[test]
    fn test_issue4_zero_delta_rejected_and_does_not_panic() {
        assert!(parse_by_day("0MO").is_err());
    }

    #[test]
    fn test_issue5_utf8_input_does_not_panic() {
        assert!(parse_by_day("1€").is_err());
        assert!(parse_by_day("1ÖM").is_err());
        assert!(parse_by_day("1MO,INVALID").is_err());
    }
}
