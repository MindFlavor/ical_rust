use crate::by_day::WeekdayRule;
use crate::errors::{DateIntersectError, SubstitutionError, TzIdDateTimeParseError};
use crate::property::split_outside_quotes;
use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime,
    TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, Eq)]
pub enum DateOrDateTime {
    WholeDay(NaiveDate),
    DateTime { utc: DateTime<Utc>, tz: Option<Tz> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventOverlap {
    FinishesPast,
    StartSameDayEndsSameDay,
    StartsSameDayEndsFuture,
    StartsPastEndsSameDay,
    StartsPastEndsFuture,
    StartsFuture,
}

impl From<NaiveDate> for DateOrDateTime {
    fn from(date: NaiveDate) -> Self {
        Self::WholeDay(date)
    }
}

impl From<DateTime<Utc>> for DateOrDateTime {
    fn from(utc: DateTime<Utc>) -> Self {
        Self::DateTime { utc, tz: None }
    }
}

impl From<DateTime<Tz>> for DateOrDateTime {
    fn from(dt: DateTime<Tz>) -> Self {
        Self::DateTime {
            utc: dt.with_timezone(&Utc),
            tz: Some(dt.timezone()),
        }
    }
}

impl FromStr for DateOrDateTime {
    type Err = TzIdDateTimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<&str> for DateOrDateTime {
    type Error = TzIdDateTimeParseError;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
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

        if let Some(colon_idx) = colon_idx {
            let params_str = &line[..colon_idx];
            let value = &line[colon_idx + 1..];
            let mut params = Vec::new();
            for part in split_outside_quotes(params_str, b';') {
                if let Some((k, v)) = part.split_once('=') {
                    params.push((k, v.trim_matches('"')));
                } else {
                    return Err(TzIdDateTimeParseError::MissingTZIDToken);
                }
            }
            Self::from_params_and_value(&params, value)
        } else if line.contains('=') {
            Err(TzIdDateTimeParseError::MissingDateTimePart)
        } else {
            Ok(Self::parse_value_str(line)?)
        }
    }
}

impl DateOrDateTime {
    pub fn from_params_and_value<K: AsRef<str>, V: AsRef<str>>(
        params: &[(K, V)],
        value: &str,
    ) -> Result<Self, TzIdDateTimeParseError> {
        Self::from_params_and_value_with_timezones(params, value, &[])
    }

    pub fn from_params_and_value_with_timezones<K: AsRef<str>, V: AsRef<str>>(
        params: &[(K, V)],
        value: &str,
        timezones: &[crate::vtimezone::VTimezone],
    ) -> Result<Self, TzIdDateTimeParseError> {
        let value_type = params
            .iter()
            .find(|(k, _)| k.as_ref().eq_ignore_ascii_case("VALUE"))
            .map(|(_, v)| v.as_ref());
        let tzid = params
            .iter()
            .find(|(k, _)| k.as_ref().eq_ignore_ascii_case("TZID"))
            .map(|(_, v)| v.as_ref());

        if value_type.is_some_and(|v| v.eq_ignore_ascii_case("DATE")) {
            let date = NaiveDate::parse_from_str(value, "%Y%m%d")?;
            Ok(Self::WholeDay(date))
        } else if let Some(tz_str) = tzid {
            if tz_str.is_empty() {
                return Err(TzIdDateTimeParseError::MissingTimeZonePart);
            }
            if value.is_empty() {
                return Err(TzIdDateTimeParseError::MissingDateTimePart);
            }
            let naive_dt = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")?;

            if let Ok(tz) = tz_str.parse::<Tz>() {
                if let LocalResult::Single(d) = tz.from_local_datetime(&naive_dt) {
                    Ok(Self::DateTime {
                        utc: d.with_timezone(&Utc),
                        tz: Some(tz),
                    })
                } else {
                    Err(TzIdDateTimeParseError::AmbiguousTimeZone)
                }
            } else if let Some(vtz) = timezones
                .iter()
                .find(|vtz| vtz.tz_id.eq_ignore_ascii_case(tz_str))
            {
                let offset = vtz
                    .offset_at(naive_dt)
                    .ok_or_else(|| TzIdDateTimeParseError::InvalidTimeZone(tz_str.to_owned()))?;
                if let LocalResult::Single(d) = offset.from_local_datetime(&naive_dt) {
                    Ok(Self::DateTime {
                        utc: d.with_timezone(&Utc),
                        tz: None,
                    })
                } else {
                    Err(TzIdDateTimeParseError::AmbiguousTimeZone)
                }
            } else {
                Err(TzIdDateTimeParseError::InvalidTimeZone(
                    "failed to parse timezone".to_owned(),
                ))
            }
        } else if value_type.is_some_and(|v| v.eq_ignore_ascii_case("DATE-TIME"))
            || params.is_empty()
        {
            Ok(Self::parse_value_str(value)?)
        } else {
            Err(TzIdDateTimeParseError::MissingTZIDToken)
        }
    }
    #[must_use]
    pub fn whole_day(date: NaiveDate) -> Self {
        Self::WholeDay(date)
    }

    #[must_use]
    pub fn date_time(utc: DateTime<Utc>) -> Self {
        Self::DateTime { utc, tz: None }
    }

    #[must_use]
    pub fn date_time_with_tz(utc: DateTime<Utc>, tz: Tz) -> Self {
        Self::DateTime { utc, tz: Some(tz) }
    }

    #[must_use]
    pub fn timezone(self) -> Option<Tz> {
        match self {
            Self::WholeDay(_) => None,
            Self::DateTime { tz, .. } => tz,
        }
    }

    #[must_use]
    pub fn date_naive(self) -> NaiveDate {
        match self {
            Self::WholeDay(d) => d,
            Self::DateTime { utc, .. } => utc.date_naive(),
        }
    }

    pub(crate) fn parse_value_str(s: &str) -> Result<Self, chrono::ParseError> {
        if s.len() == 8 {
            let date = NaiveDate::parse_from_str(s, "%Y%m%d")?;
            Ok(Self::WholeDay(date))
        } else {
            let utc = if s.ends_with('Z') {
                DateTime::<FixedOffset>::parse_from_str(s, "%Y%m%dT%H%M%S%#z")?.with_timezone(&Utc)
            } else {
                let a = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S")?;
                Local
                    .from_local_datetime(&a)
                    .earliest()
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|| Utc.from_utc_datetime(&a))
            };
            Ok(Self::DateTime { utc, tz: None })
        }
    }

    pub fn substitute(
        self,
        year: Option<i32>,
        month: Option<u32>,
        day: Option<u32>,
        hour: Option<u32>,
        minute: Option<u32>,
        second: Option<u32>,
    ) -> Result<Self, SubstitutionError> {
        let date = NaiveDate::from_ymd_opt(
            year.unwrap_or_else(|| self.year()),
            month.unwrap_or_else(|| self.month()),
            day.unwrap_or_else(|| self.day()),
        )
        .ok_or(SubstitutionError::InvalidDateOrTime)?;

        Ok(match self {
            DateOrDateTime::WholeDay(_) => {
                if hour.is_some() || minute.is_some() || second.is_some() {
                    return Err(SubstitutionError::ConstructingDateTimeBySubstitutingWholeDay);
                }
                DateOrDateTime::WholeDay(date)
            }
            DateOrDateTime::DateTime { utc, tz } => {
                let dt = date
                    .and_hms_opt(
                        hour.unwrap_or_else(|| utc.hour()),
                        minute.unwrap_or_else(|| utc.minute()),
                        second.unwrap_or_else(|| utc.second()),
                    )
                    .ok_or(SubstitutionError::InvalidDateOrTime)?
                    .and_utc();
                DateOrDateTime::DateTime { utc: dt, tz }
            }
        })
    }

    #[must_use]
    pub fn next_by_day(self, by_day: &[WeekdayRule]) -> Self {
        let weekdays: Vec<Weekday> = by_day.iter().map(|r| r.weekday).collect();
        self.next_weekdays(&weekdays)
    }

    #[must_use]
    pub fn matching_days_in_month(
        self,
        year: i32,
        month: u32,
        rules: &[WeekdayRule],
    ) -> Vec<DateOrDateTime> {
        let mut days_in_month = Vec::with_capacity(31);
        for d in 1..=31 {
            if let Ok(dt) = self.substitute(Some(year), Some(month), Some(d), None, None, None) {
                days_in_month.push(dt);
            } else {
                break;
            }
        }

        let mut result = Vec::new();
        for rule in rules {
            let matching: Vec<DateOrDateTime> = days_in_month
                .iter()
                .copied()
                .filter(|dt| dt.date_naive().weekday() == rule.weekday)
                .collect();

            match rule.offset {
                None => result.extend(matching),
                Some(offset) if offset > 0 => {
                    if let Some(&dt) = matching.get(offset as usize - 1) {
                        result.push(dt);
                    }
                }
                Some(offset) if offset < 0 => {
                    let abs = offset.unsigned_abs() as usize;
                    if matching.len() >= abs {
                        result.push(matching[matching.len() - abs]);
                    }
                }
                Some(_) => {}
            }
        }

        result.sort();
        result.dedup();
        result
    }

    #[must_use]
    pub fn next_weekday(self, weekday: Weekday) -> Self {
        self.next_weekdays(&[weekday])
    }

    #[must_use]
    pub fn next_weekdays(self, weekdays: &[Weekday]) -> Self {
        let mut ret = self + Duration::days(1);

        while !weekdays
            .iter()
            .any(|weekday| ret.date_naive().weekday() == *weekday)
        {
            ret = ret + Duration::days(1);
        }

        ret
    }

    #[must_use]
    pub fn move_by_delta(self, rule: &WeekdayRule) -> DateOrDateTime {
        match rule.offset {
            Some(0) => self,
            Some(_) => self
                .matching_days_in_month(self.year(), self.month(), &[*rule])
                .into_iter()
                .next()
                .unwrap_or(self),
            None => self.next_weekday(rule.weekday),
        }
    }

    #[must_use]
    pub fn equals_date_time(self, date_time: DateTime<Utc>) -> bool {
        self.as_datetime() == date_time
    }

    #[must_use]
    pub fn inc_month(self, increment: u32) -> Self {
        let total_months = (self.month() - 1) + increment;
        let final_month = (total_months % 12) + 1;
        let delta_years = total_months / 12;

        let year = self.year() + delta_years as i32;
        let month = final_month;
        let mut d = self.day();

        // we need to clamp because some months do not have all the dates. For example, february
        // does not have 30, 31 (and sometimes not even 29).
        let date = loop {
            if let Some(valid) = NaiveDate::from_ymd_opt(year, month, d) {
                break valid;
            }
            d -= 1;
        };

        match self {
            DateOrDateTime::WholeDay(_) => DateOrDateTime::WholeDay(date),
            DateOrDateTime::DateTime { utc, tz } => DateOrDateTime::DateTime {
                utc: date
                    .and_hms_opt(utc.hour(), utc.minute(), utc.second())
                    .unwrap()
                    .and_utc(),
                tz,
            },
        }
    }

    #[must_use]
    pub fn inc_year(self, increment: u32) -> DateOrDateTime {
        let mut day = self.day();
        let year = self.year() + increment as i32;
        let month = self.month();
        let date = loop {
            if let Some(valid) = NaiveDate::from_ymd_opt(year, month, day) {
                break valid;
            }
            day -= 1;
        };

        match self {
            DateOrDateTime::WholeDay(_) => Self::WholeDay(date),
            DateOrDateTime::DateTime { utc, tz } => Self::DateTime {
                utc: date
                    .and_hms_opt(utc.hour(), utc.minute(), utc.second())
                    .unwrap()
                    .and_utc(),
                tz,
            },
        }
    }

    #[must_use]
    pub fn year(self) -> i32 {
        self.date_naive().year()
    }

    #[must_use]
    pub fn month(self) -> u32 {
        self.date_naive().month()
    }

    #[must_use]
    pub fn day(self) -> u32 {
        self.date_naive().day()
    }

    #[must_use]
    pub fn hour(self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_) => 0,
            DateOrDateTime::DateTime { utc, .. } => utc.hour(),
        }
    }

    #[must_use]
    pub fn minute(self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_) => 0,
            DateOrDateTime::DateTime { utc, .. } => utc.minute(),
        }
    }

    #[must_use]
    pub fn second(self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_) => 0,
            DateOrDateTime::DateTime { utc, .. } => utc.second(),
        }
    }

    pub fn intersects(
        self,
        dt_start: DateOrDateTime,
        dt_end: DateOrDateTime,
    ) -> Result<EventOverlap, DateIntersectError> {
        log::trace!("intersects({self:?}, dt_start == {dt_start:?}, dt_end == {dt_end:?})");

        let day = self.date_naive();
        let d_start = dt_start.date_naive();
        let d_end = dt_end.date_naive();

        if d_start > d_end {
            return Err(DateIntersectError::StartDateAfterEndDate);
        }

        match (d_start.cmp(&day), d_end.cmp(&day)) {
            (Ordering::Less, Ordering::Less) => Ok(EventOverlap::FinishesPast),
            (Ordering::Less, Ordering::Equal) => Ok(EventOverlap::StartsPastEndsSameDay),
            (Ordering::Less, Ordering::Greater) => Ok(EventOverlap::StartsPastEndsFuture),
            (Ordering::Equal, Ordering::Less) => Err(DateIntersectError::StartDateAfterEndDate),
            (Ordering::Equal, Ordering::Equal) => Ok(EventOverlap::StartSameDayEndsSameDay),
            (Ordering::Equal, Ordering::Greater) => Ok(EventOverlap::StartsSameDayEndsFuture),
            (Ordering::Greater, _) => Ok(EventOverlap::StartsFuture),
        }
    }

    #[must_use]
    pub fn succ_day(self) -> DateOrDateTime {
        self + Duration::days(1)
    }

    #[must_use]
    pub fn as_datetime(self) -> DateTime<Utc> {
        match self {
            DateOrDateTime::WholeDay(day) => day.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            DateOrDateTime::DateTime { utc, .. } => utc,
        }
    }
}

impl PartialEq for DateOrDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.as_datetime() == other.as_datetime()
    }
}

impl Hash for DateOrDateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_datetime().hash(state);
    }
}

impl Ord for DateOrDateTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_datetime().cmp(&other.as_datetime())
    }
}

impl PartialOrd for DateOrDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Sub for DateOrDateTime {
    type Output = chrono::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.as_datetime() - rhs.as_datetime()
    }
}

impl Add<Duration> for DateOrDateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => Self::WholeDay(day + rhs),
            DateOrDateTime::DateTime { utc, tz } => Self::DateTime { utc: utc + rhs, tz },
        }
    }
}

impl Sub<Duration> for DateOrDateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => Self::WholeDay(day - rhs),
            DateOrDateTime::DateTime { utc, tz } => Self::DateTime { utc: utc - rhs, tz },
        }
    }
}

#[cfg(test)]
#[path = "tests/date_or_date_time.rs"]
mod tests;
