use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::{fmt::Debug, str::FromStr};
use thiserror::Error;

use crate::DateOrDateTime;

#[derive(Error, Debug)]
pub enum TzIdDateTimeFormatError {
    #[error("Parse date time error")]
    ParseIntError(#[from] chrono::ParseError),
    #[error("Ambiguous timezone")]
    AmbiguousTimeZone,
    #[error("Missing TZID= token")]
    MissingTZIDToken,
}

#[derive(Debug, Clone)]
pub struct TzIdDateTime {
    pub time_zone: Tz,
    pub date_time: DateOrDateTime,
}

impl FromStr for TzIdDateTime {
    type Err = TzIdDateTimeFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl<T: TimeZone> From<DateTime<T>> for TzIdDateTime {
    fn from(dt: DateTime<T>) -> Self {
        Self {
            time_zone: chrono_tz::UTC,
            date_time: DateOrDateTime::DateTime(dt.with_timezone(&Utc)),
        }
    }
}

impl TryFrom<&str> for TzIdDateTime {
    type Error = TzIdDateTimeFormatError;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if let Some(line) = line.strip_prefix("TZID=") {
            let mut tokens = line.split(':');

            let tz: Tz = tokens.next().unwrap().parse().unwrap();

            let date_time = tokens.next().unwrap();
            let date_time = NaiveDateTime::parse_from_str(date_time, "%Y%m%dT%H%M%S")?;

            if let LocalResult::Single(d) = tz.from_local_datetime(&date_time) {
                Ok(Self {
                    time_zone: tz,
                    date_time: DateOrDateTime::DateTime(d.with_timezone(&Utc)),
                })
            } else {
                Err(TzIdDateTimeFormatError::AmbiguousTimeZone)
            }
        } else if let Some(line) = line.strip_prefix("VALUE=DATE:") {
            let date = Utc.from_utc_datetime(&NaiveDateTime::parse_from_str(line, "%Y%m%d")?);
            Ok(Self {
                time_zone: chrono_tz::UTC,
                date_time: DateOrDateTime::WholeDay(date),
            })
        } else {
            Err(TzIdDateTimeFormatError::MissingTZIDToken)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TzIdDateTime;

    #[test]
    fn parse_00() {
        let s = "TZID=Europe/Rome:20220106T154000";

        let _: TzIdDateTime = s.try_into().unwrap();
    }

    #[test]
    fn parse_01() {
        let s = "TZID=Europe/Rome:20211006T170000";

        let _: TzIdDateTime = s.try_into().unwrap();
    }
}
