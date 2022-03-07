use chrono::Weekday;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ByDayParseError {
    #[error("Invalid weekday {w:?})")]
    InvalidWeekday { w: String },
    #[error("Invalid delta")]
    InvalidDelta(#[from] std::num::ParseIntError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Delta {
    pub delta: i32,
    pub weekday: Weekday,
}

impl Delta {
    pub fn new(delta: i32, weekday: Weekday) -> Self {
        Self { delta, weekday }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ByDay {
    Simple(Vec<Weekday>),
    Delta(Delta),
}

impl FromStr for ByDay {
    type Err = ByDayParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = s
            .split(',')
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        if tokens[0].len() > 2 {
            Ok(ByDay::Delta(tokens[0].parse()?))
        } else {
            Ok(Self::Simple(
                tokens
                    .into_iter()
                    .map(to_chrono_weekday)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
    }
}

impl FromStr for Delta {
    type Err = ByDayParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let weekday = to_chrono_weekday(&s[s.len() - 2..])?;
        let delta: i32 = s[..s.len() - 2].parse()?;
        Ok(Self { delta, weekday })
    }
}

fn to_chrono_weekday(s: &str) -> Result<chrono::Weekday, ByDayParseError> {
    match s {
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
        let _: ByDay = s.parse().unwrap();
    }

    #[test]
    fn parse_delta() {
        let _: ByDay = "-20MO".parse().unwrap();
        let _: ByDay = "30FR".parse().unwrap();
    }
}
