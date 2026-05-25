use std::str::FromStr;
use crate::errors::FrequencyParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Frequency {
    Yearly,
    Monthly,
    Weekly,
    Daily,
}

impl FromStr for Frequency {
    type Err = FrequencyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "YEARLY" => Ok(Frequency::Yearly),
            "MONTHLY" => Ok(Frequency::Monthly),
            "WEEKLY" => Ok(Frequency::Weekly),
            "DAILY" => Ok(Frequency::Daily),
            _ => Err(FrequencyParseError::UnrecognizedFrequency { freq: s.to_owned() }),
        }
    }
}
