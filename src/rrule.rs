use crate::{
    by_day::{ByDay, ByDayParseError},
    date_or_date_time::DateOrDateTime,
    frequency::{Frequency, FrequencyParseError},
    string_to_date_or_datetime,
};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RRuleParseError {
    #[error("Generic error")]
    Generic,
    #[error("Frequency parse error {err:?})")]
    FrequencyParseError { err: FrequencyParseError },
    #[error("Missing frequency token {line:?})")]
    MissingFrequencyToken { line: String },
    #[error("Missing next token after BYMONTH {line:?})")]
    MissingrNextTokenAfterByMonth { line: String },
    #[error("ParseIntError")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("ParseDateOrDatetTimeError")]
    ParseDateOrDatetTimeError(#[from] chrono::ParseError),
    #[error("Missing either BYDAY or BYMONTHDAY {line:?})")]
    MissingByDayOrByMonthDayError { line: String },
    #[error("Missing BYDAY {line:?})")]
    MissingByDayError { line: String },
    #[error("ByDayParserError ({error:?}) line == {line:?}")]
    ByDayParserError {
        error: ByDayParseError,
        line: String,
    },
}

pub trait Options: std::fmt::Debug {
    fn common_options(&self) -> &CommonOptions;

    fn is_out_of_count(&self, count: u32) -> bool {
        log::trace!(
            "is_out_of_count(self == {:?}, count == {}) called",
            self,
            count
        );
        self.common_options()
            .count
            .map(|rrule_count| count >= rrule_count)
            .unwrap_or(false)
    }

    fn is_expired(&self, dt: DateOrDateTime) -> bool {
        log::debug!("is_expired(self == {:?}, dt == {:?}) called", self, dt);
        self.common_options()
            .until
            .map(|until| dt > until)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RRule {
    Yearly(Yearly),
    YearlyByMonthByMonthDay(YearlyByMonthByMonthDay),
    YearlyByMonthByDay(YearlyByMonthByDay),
    MonthlyByMonthDay(MonthlyByMonthDay),
    MonthlyByDay(MonthlyByDay),
    WeeklyByDay(WeeklyByDay),
    Weekly(Weekly),
    Daily(Daily),
}

impl FromStr for RRule {
    type Err = RRuleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split(';');
        let freq = tokens
            .next()
            .ok_or_else(|| RRuleParseError::MissingFrequencyToken { line: s.to_owned() })?;
        let frequency = if let Some(freq) = freq.strip_prefix("FREQ=") {
            freq.parse()
                .map_err(|err| RRuleParseError::FrequencyParseError { err })?
        } else {
            return Err(RRuleParseError::MissingFrequencyToken { line: s.to_owned() });
        };

        let tokens: Vec<_> = tokens.collect();
        let interval: Option<u32> = tokens
            .iter()
            .find(|item| item.starts_with("INTERVAL="))
            .map(|item| &item["INTERVAL=".len()..])
            .map(|item| item.parse())
            .transpose()?;

        let until: Option<DateOrDateTime> = tokens
            .iter()
            .find(|item| item.starts_with("UNTIL="))
            .map(|item| &item["UNTIL=".len()..])
            .map(string_to_date_or_datetime)
            .transpose()?;

        let count = tokens
            .iter()
            .find(|item| item.starts_with("COUNT="))
            .map(|item| &item["COUNT=".len()..])
            .map(|s| s.parse())
            .transpose()?;

        let by_month: Option<u8> = tokens
            .iter()
            .find(|item| item.starts_with("BYMONTH="))
            .map(|item| &item["BYMONTH=".len()..])
            .map(|s| s.parse())
            .transpose()?;

        let by_month_day: Option<u8> = tokens
            .iter()
            .find(|item| item.starts_with("BYMONTHDAY="))
            .map(|item| &item["BYMONTHDAY=".len()..])
            .map(|s| s.parse())
            .transpose()?;

        let by_day: Option<ByDay> = tokens
            .iter()
            .find(|item| item.starts_with("BYDAY="))
            .map(|item| &item["BYDAY=".len()..])
            .map(|s| s.parse())
            .transpose()
            .map_err(|error| RRuleParseError::ByDayParserError {
                error,
                line: s.to_owned(),
            })?;

        Ok(match frequency {
            Frequency::Yearly => {
                if let Some(by_month) = by_month {
                    if let Some(by_month_day) = by_month_day {
                        Self::YearlyByMonthByMonthDay(YearlyByMonthByMonthDay {
                            month: by_month,
                            month_day: by_month_day,
                            common_options: CommonOptions::new(s, until, interval, count),
                        })
                    } else if let Some(by_day) = by_day {
                        Self::YearlyByMonthByDay(YearlyByMonthByDay {
                            month: by_month,
                            day: by_day,
                            common_options: CommonOptions::new(s, until, interval, count),
                        })
                    } else {
                        return Err(RRuleParseError::MissingrNextTokenAfterByMonth {
                            line: s.to_owned(),
                        });
                    }
                } else {
                    // we ignore WKST
                    Self::Yearly(Yearly {
                        common_options: CommonOptions::new(s, until, interval, count),
                    })
                }
            }

            Frequency::Monthly => {
                if let Some(by_month_day) = by_month_day {
                    Self::MonthlyByMonthDay(MonthlyByMonthDay {
                        month_day: by_month_day,
                        common_options: CommonOptions::new(s, until, interval, count),
                    })
                } else if let Some(by_day) = by_day {
                    Self::MonthlyByDay(MonthlyByDay {
                        day: by_day,
                        common_options: CommonOptions::new(s, until, interval, count),
                    })
                } else {
                    return Err(RRuleParseError::MissingByDayOrByMonthDayError {
                        line: s.to_owned(),
                    });
                }
            }

            Frequency::Weekly => {
                if let Some(day) = by_day {
                    Self::WeeklyByDay(WeeklyByDay {
                        day,
                        common_options: CommonOptions::new(s, until, interval, count),
                    })
                } else {
                    Self::Weekly(Weekly {
                        common_options: CommonOptions::new(s, until, interval, count),
                    })
                }
            }

            Frequency::Daily => Self::Daily(Daily {
                common_options: CommonOptions::new(s, until, interval, count),
            }),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic {
    pub frequency: Frequency,
    pub raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CommonOptions {
    pub raw: String,
    pub until: Option<DateOrDateTime>,
    pub interval: Option<u32>,
    pub count: Option<u32>,
}

impl CommonOptions {
    fn new(
        raw: impl Into<String>,
        until: Option<DateOrDateTime>,
        interval: Option<u32>,
        count: Option<u32>,
    ) -> Self {
        Self {
            raw: raw.into(),
            until,
            interval,
            count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Yearly {
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YearlyByMonthByMonthDay {
    pub month: u8,
    pub month_day: u8,
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YearlyByMonthByDay {
    pub month: u8,
    pub day: ByDay,
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonthlyByMonthDay {
    pub month_day: u8,
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonthlyByDay {
    pub day: ByDay,
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WeeklyByDay {
    pub day: ByDay,
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Weekly {
    pub common_options: CommonOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Daily {
    pub common_options: CommonOptions,
}

impl Options for Yearly {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for YearlyByMonthByDay {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for YearlyByMonthByMonthDay {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for MonthlyByDay {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for MonthlyByMonthDay {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for Weekly {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for WeeklyByDay {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for Daily {
    fn common_options(&self) -> &CommonOptions {
        &self.common_options
    }
}

impl Options for RRule {
    fn common_options(&self) -> &CommonOptions {
        match self {
            RRule::Yearly(rrule) => &rrule.common_options,
            RRule::YearlyByMonthByDay(rrule) => &rrule.common_options,
            RRule::YearlyByMonthByMonthDay(rrule) => &rrule.common_options,
            RRule::MonthlyByMonthDay(rrule) => &rrule.common_options,
            RRule::MonthlyByDay(rrule) => &rrule.common_options,
            RRule::WeeklyByDay(rrule) => &rrule.common_options,
            RRule::Weekly(rrule) => &rrule.common_options,
            RRule::Daily(rrule) => &rrule.common_options,
        }
    }
}
