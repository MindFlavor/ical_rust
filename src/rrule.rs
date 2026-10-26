use crate::{
    by_day::{WeekdayRule, parse_by_day, to_chrono_weekday, weekday_to_ical_str},
    date_or_date_time::DateOrDateTime,
    errors::RRuleParseError,
    frequency::Frequency,
    string_to_date_or_datetime,
};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RRule {
    pub frequency: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateOrDateTime>,
    pub by_month: Vec<u8>,
    pub by_month_day: Vec<i8>,
    pub by_day: Vec<WeekdayRule>,
    pub by_set_pos: Vec<i16>,
    pub wkst: Option<chrono::Weekday>,
}

impl RRule {
    pub fn is_out_of_count(&self, count: u32) -> bool {
        log::trace!(
            "is_out_of_count(self == {:?}, count == {}) called",
            self,
            count
        );
        self.count
            .map(|rrule_count| count >= rrule_count)
            .unwrap_or(false)
    }

    pub fn is_expired(&self, dt: DateOrDateTime) -> bool {
        log::debug!("is_expired(self == {:?}, dt == {:?}) called", self, dt);
        self.until.map(|until| dt > until).unwrap_or(false)
    }
}

impl fmt::Display for RRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FREQ={}", self.frequency)?;
        if self.interval != 1 {
            write!(f, ";INTERVAL={}", self.interval)?;
        }
        if let Some(count) = self.count {
            write!(f, ";COUNT={count}")?;
        }
        if let Some(until) = self.until {
            match until {
                DateOrDateTime::WholeDay(d) => write!(f, ";UNTIL={}", d.format("%Y%m%d"))?,
                DateOrDateTime::DateTime { utc, .. } => {
                    write!(f, ";UNTIL={}", utc.format("%Y%m%dT%H%M%SZ"))?
                }
            }
        }
        if !self.by_month.is_empty() {
            let months = self
                .by_month
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(f, ";BYMONTH={months}")?;
        }
        if !self.by_month_day.is_empty() {
            let month_days = self
                .by_month_day
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(f, ";BYMONTHDAY={month_days}")?;
        }
        if !self.by_day.is_empty() {
            let days = self
                .by_day
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(f, ";BYDAY={days}")?;
        }
        if !self.by_set_pos.is_empty() {
            let pos = self
                .by_set_pos
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(f, ";BYSETPOS={pos}")?;
        }
        if let Some(wkst) = self.wkst {
            write!(f, ";WKST={}", weekday_to_ical_str(wkst))?;
        }
        Ok(())
    }
}

impl FromStr for RRule {
    type Err = RRuleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<_> = s.split(';').collect();
        let freq = tokens
            .iter()
            .find_map(|item| item.strip_prefix("FREQ="))
            .ok_or_else(|| RRuleParseError::MissingFrequencyToken { line: s.to_owned() })?;
        let frequency = freq
            .parse()
            .map_err(|err| RRuleParseError::FrequencyParseError { err })?;

        let interval: u32 = tokens
            .iter()
            .find_map(|item| item.strip_prefix("INTERVAL="))
            .map(|item| item.parse())
            .transpose()?
            .unwrap_or(1);

        if interval == 0 {
            return Err(RRuleParseError::InvalidInterval { interval });
        }

        let until: Option<DateOrDateTime> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("UNTIL="))
            .map(string_to_date_or_datetime)
            .transpose()?;

        let count: Option<u32> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("COUNT="))
            .map(|item| item.parse())
            .transpose()?;

        if count.is_some() && until.is_some() {
            return Err(RRuleParseError::BothCountAndUntilSpecified);
        }

        let by_month: Vec<u8> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("BYMONTH="))
            .map(|item| {
                item.split(',')
                    .map(|m| m.parse::<u8>())
                    .collect::<Result<Vec<u8>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        for &month in &by_month {
            if !(1..=12).contains(&month) {
                return Err(RRuleParseError::InvalidByMonth { month });
            }
        }

        let by_month_day: Vec<i8> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("BYMONTHDAY="))
            .map(|item| {
                item.split(',')
                    .map(|d| d.parse::<i8>())
                    .collect::<Result<Vec<i8>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        for &day in &by_month_day {
            if day == 0 || !(-31..=31).contains(&day) {
                return Err(RRuleParseError::InvalidByMonthDay { day });
            }
        }

        let by_day: Vec<WeekdayRule> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("BYDAY="))
            .map(parse_by_day)
            .transpose()
            .map_err(|error| RRuleParseError::ByDayParserError {
                error,
                line: s.to_owned(),
            })?
            .unwrap_or_default();

        let by_set_pos: Vec<i16> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("BYSETPOS="))
            .map(|item| {
                item.split(',')
                    .map(|p| p.parse::<i16>())
                    .collect::<Result<Vec<i16>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        for &pos in &by_set_pos {
            if pos == 0 || !(-366..=366).contains(&pos) {
                return Err(RRuleParseError::InvalidBySetPos { pos });
            }
        }

        let wkst: Option<chrono::Weekday> = tokens
            .iter()
            .find_map(|item| item.strip_prefix("WKST="))
            .map(to_chrono_weekday)
            .transpose()
            .map_err(|error| RRuleParseError::ByDayParserError {
                error,
                line: s.to_owned(),
            })?;

        Ok(Self {
            frequency,
            interval,
            count,
            until,
            by_month,
            by_month_day,
            by_day,
            by_set_pos,
            wkst,
        })
    }
}
