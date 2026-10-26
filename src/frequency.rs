use crate::errors::FrequencyParseError;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Frequency {
    Yearly,
    Monthly,
    Weekly,
    Daily,
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Frequency::Yearly => "YEARLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Daily => "DAILY",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Frequency {
    type Err = FrequencyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("YEARLY") {
            Ok(Frequency::Yearly)
        } else if s.eq_ignore_ascii_case("MONTHLY") {
            Ok(Frequency::Monthly)
        } else if s.eq_ignore_ascii_case("WEEKLY") {
            Ok(Frequency::Weekly)
        } else if s.eq_ignore_ascii_case("DAILY") {
            Ok(Frequency::Daily)
        } else {
            Err(FrequencyParseError::UnrecognizedFrequency { freq: s.to_owned() })
        }
    }
}
