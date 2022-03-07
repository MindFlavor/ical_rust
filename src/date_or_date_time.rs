use crate::by_day::{ByDay, Delta};
use chrono::{Date, DateTime, Datelike, Duration, LocalResult, TimeZone, Timelike, Utc, Weekday};
use std::{
    cmp::Ordering,
    ops::{Add, Sub},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SubstitutionError {
    #[error("Cannot construct a date time variant by substituting a Whole day")]
    ConstructingDateTimeBySubstitutingWholeDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateOrDateTime {
    WholeDay(Date<Utc>),
    DateTime(DateTime<Utc>),
}

#[derive(Error, Debug)]
pub enum DateIntersectError {
    #[error("Start date cannot be after end date")]
    StartDateAfterEndDate,
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

impl DateOrDateTime {
    pub fn substitute(
        self,
        year: Option<i32>,
        month: Option<u32>,
        day: Option<u32>,
        hour: Option<u32>,
        minute: Option<u32>,
        second: Option<u32>,
    ) -> Result<Self, SubstitutionError> {
        let date = Utc.ymd(
            year.unwrap_or_else(|| self.year()),
            month.unwrap_or_else(|| self.month()),
            day.unwrap_or_else(|| self.day()),
        );

        Ok(match self {
            DateOrDateTime::WholeDay(_) => {
                if hour.is_some() || minute.is_some() || second.is_some() {
                    return Err(SubstitutionError::ConstructingDateTimeBySubstitutingWholeDay);
                }
                date.into()
            }
            DateOrDateTime::DateTime(dt) => date
                .and_hms(
                    hour.unwrap_or_else(|| dt.hour()),
                    minute.unwrap_or_else(|| dt.minute()),
                    second.unwrap_or_else(|| dt.second()),
                )
                .into(),
        })
    }

    pub fn substitute_time_with(self, time: impl Into<DateOrDateTime>) -> Self {
        match time.into() {
            DateOrDateTime::WholeDay(_) => self,
            DateOrDateTime::DateTime(dt) => self
                .date()
                .and_hms(dt.hour(), dt.minute(), dt.second())
                .into(),
        }
    }

    pub fn next_by_day(self, by_day: &ByDay) -> Self {
        match by_day {
            ByDay::Delta(delta) => self.move_by_delta(delta),
            ByDay::Simple(weekdays) => self.next_weekdays(weekdays),
        }
    }

    pub fn next_weekday(self, weekday: Weekday) -> Self {
        self.next_weekdays(&[weekday])
    }

    pub fn next_weekdays(self, weekdays: &[Weekday]) -> Self {
        let mut ret = self + Duration::days(1);

        while !weekdays
            .iter()
            .any(|weekday| ret.date().weekday() == *weekday)
        {
            ret = ret + Duration::days(1);
        }

        ret
    }

    pub fn move_by_delta(self, delta: &Delta) -> DateOrDateTime {
        let month_start = self
            .substitute(None, None, Some(1), None, None, None)
            .unwrap();

        let month_end = self
            .substitute(
                Some(if self.month() == 12 {
                    self.year() + 1
                } else {
                    self.year()
                }),
                Some(if self.month() == 12 {
                    1
                } else {
                    self.month() + 1
                }),
                Some(1),
                None,
                None,
                None,
            )
            .unwrap()
            .sub(Duration::days(1));

        let mut current_delta = delta.delta.abs() - 1;
        let increment = Duration::days(delta.delta as i64 / delta.delta.abs() as i64);
        let mut current_day = if increment.num_days() == 1 {
            month_start
        } else {
            month_end
        };

        loop {
            log::debug!(
                "current_delta = {}, increment = {:?}, current_day = {:?}",
                current_delta,
                increment,
                current_day
            );

            if current_day.date().weekday() == delta.weekday {
                if current_delta == 0 {
                    return current_day;
                } else {
                    current_delta -= 1;
                }
            }

            current_day = current_day + increment;
        }
    }

    pub fn equals_date(self, date: Date<Utc>) -> bool {
        match self {
            DateOrDateTime::WholeDay(d) => date == d,
            DateOrDateTime::DateTime(dt) => date.and_hms(0, 0, 0) == dt,
        }
    }

    pub fn equals_date_time(self, date_time: DateTime<Utc>) -> bool {
        match self {
            DateOrDateTime::WholeDay(d) => d.and_hms(0, 0, 0) == date_time,
            DateOrDateTime::DateTime(dt) => dt == date_time,
        }
    }

    pub fn inc_month(self, increment: u32) -> Self {
        let delta_final_months = self.month() + increment;
        let delta_years = delta_final_months / 12;
        let final_month = std::cmp::max(delta_final_months - delta_years * 12, 1);

        let mut year = self.year() + delta_years as i32;
        let mut month = final_month;
        let day = self.day();

        // we need to loop because some months do not have all the dates. For example, february is
        // does not have 30,31 (and sometimes not even 29).
        let date = {
            let mut date = Utc.ymd_opt(year, month, day);
            while matches!(date, LocalResult::None) {
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }

                date = Utc.ymd_opt(year, month, day);
            }
            date.unwrap()
        };

        match self {
            DateOrDateTime::WholeDay(_) => date.into(),
            DateOrDateTime::DateTime(dt) => {
                date.and_hms(dt.hour(), dt.minute(), dt.second()).into()
            }
        }
    }

    pub fn inc_year(&self, increment: u32) -> DateOrDateTime {
        match self {
            DateOrDateTime::WholeDay(d) => {
                let d = Utc.ymd(d.year() + increment as i32, d.month(), d.day());
                Self::WholeDay(d)
            }
            DateOrDateTime::DateTime(d) => {
                let d = Utc
                    .ymd(d.year() + increment as i32, d.month(), d.day())
                    .and_hms(d.hour(), d.minute(), d.second());
                Self::DateTime(d)
            }
        }
    }

    pub fn date(self) -> Date<Utc> {
        match self {
            DateOrDateTime::WholeDay(d) => d,
            DateOrDateTime::DateTime(dt) => dt.date(),
        }
    }

    pub fn year(&self) -> i32 {
        match self {
            DateOrDateTime::WholeDay(d) => d.year(),
            DateOrDateTime::DateTime(d) => d.year(),
        }
    }

    pub fn month(&self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(d) => d.month(),
            DateOrDateTime::DateTime(d) => d.month(),
        }
    }

    pub fn day(&self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(d) => d.day(),
            DateOrDateTime::DateTime(d) => d.day(),
        }
    }

    pub fn hour(&self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_d) => 0,
            DateOrDateTime::DateTime(d) => d.hour(),
        }
    }

    pub fn minute(&self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_d) => 0,
            DateOrDateTime::DateTime(d) => d.minute(),
        }
    }

    pub fn second(&self) -> u32 {
        match self {
            DateOrDateTime::WholeDay(_d) => 0,
            DateOrDateTime::DateTime(d) => d.second(),
        }
    }

    pub fn intersects(
        self,
        dt_start: DateOrDateTime,
        dt_end: DateOrDateTime,
    ) -> Result<EventOverlap, DateIntersectError> {
        log::trace!("intersects({:?}, {:?}, {:?})", self, dt_start, dt_end);

        match self {
            DateOrDateTime::WholeDay(day) => {
                let d_start = dt_start.date();
                let d_end = dt_end.date();

                match (d_start.cmp(&day), d_end.cmp(&day)) {
                    (Ordering::Less, Ordering::Less) => Ok(EventOverlap::FinishesPast),
                    (Ordering::Less, Ordering::Equal) => Ok(EventOverlap::StartsPastEndsSameDay),
                    (Ordering::Less, Ordering::Greater) => Ok(EventOverlap::StartsPastEndsFuture),
                    (Ordering::Equal, Ordering::Less) => {
                        Err(DateIntersectError::StartDateAfterEndDate)
                    }
                    (Ordering::Equal, Ordering::Equal) => Ok(EventOverlap::StartSameDayEndsSameDay),
                    (Ordering::Equal, Ordering::Greater) => {
                        Ok(EventOverlap::StartsSameDayEndsFuture)
                    }
                    (Ordering::Greater, _) => Ok(EventOverlap::StartsFuture),
                }
            }
            DateOrDateTime::DateTime(dt) => {
                let dt_start = match dt_start {
                    DateOrDateTime::DateTime(dt) => dt,
                    DateOrDateTime::WholeDay(d) => d.and_hms(0, 0, 0),
                };
                let dt_end = match dt_end {
                    DateOrDateTime::DateTime(dt) => dt,
                    DateOrDateTime::WholeDay(d) => d.and_hms(0, 0, 0) + Duration::days(1),
                };

                match (dt_start.cmp(&dt), dt_end.cmp(&dt)) {
                    (Ordering::Less, Ordering::Less) => Ok(EventOverlap::FinishesPast),
                    (Ordering::Less, Ordering::Equal) => Ok(EventOverlap::StartsPastEndsSameDay),
                    (Ordering::Less, Ordering::Greater) => Ok(EventOverlap::StartsPastEndsFuture),
                    (Ordering::Equal, Ordering::Less) => {
                        Err(DateIntersectError::StartDateAfterEndDate)
                    }
                    (Ordering::Equal, Ordering::Equal) => Ok(EventOverlap::StartSameDayEndsSameDay),
                    (Ordering::Equal, Ordering::Greater) => {
                        Ok(EventOverlap::StartsSameDayEndsFuture)
                    }
                    (Ordering::Greater, _) => Ok(EventOverlap::StartsFuture),
                }
            }
        }
    }
}

impl<T: chrono::TimeZone> From<Date<T>> for DateOrDateTime {
    fn from(d: Date<T>) -> Self {
        Self::WholeDay(d.with_timezone(&Utc))
    }
}

impl<T: chrono::TimeZone> From<DateTime<T>> for DateOrDateTime {
    fn from(dt: DateTime<T>) -> Self {
        Self::DateTime(dt.with_timezone(&Utc))
    }
}

impl DateOrDateTime {
    pub fn succ_day(&self) -> DateOrDateTime {
        match self {
            DateOrDateTime::WholeDay(whole) => DateOrDateTime::WholeDay(whole.succ()),
            DateOrDateTime::DateTime(dt) => {
                DateOrDateTime::DateTime(*dt + chrono::Duration::days(1))
            }
        }
    }

    pub fn as_datetime(&self) -> DateTime<Utc> {
        match self {
            DateOrDateTime::WholeDay(day) => day.and_hms(0, 0, 0),
            DateOrDateTime::DateTime(dt) => *dt,
        }
    }
}

impl PartialOrd for DateOrDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // convert in date time if necessary
        let self_dt = match self {
            DateOrDateTime::DateTime(dt) => *dt,
            DateOrDateTime::WholeDay(dt) => dt.and_hms(0, 0, 0),
        };

        let other_dt = match other {
            DateOrDateTime::DateTime(dt) => *dt,
            DateOrDateTime::WholeDay(dt) => dt.and_hms(0, 0, 0),
        };

        Some(self_dt.cmp(&other_dt))
    }
}

impl Ord for DateOrDateTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Sub for DateOrDateTime {
    type Output = chrono::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        let dt_self = match self {
            DateOrDateTime::WholeDay(d) => d.and_hms(0, 0, 0),
            DateOrDateTime::DateTime(dt) => dt,
        };

        let dt_rhs = match rhs {
            DateOrDateTime::WholeDay(d) => d.and_hms(0, 0, 0),
            DateOrDateTime::DateTime(dt) => dt,
        };

        dt_self - dt_rhs
    }
}

impl Add<Duration> for DateOrDateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => (day + rhs).into(),
            DateOrDateTime::DateTime(dt) => (dt + rhs).into(),
        }
    }
}

impl Sub<Duration> for DateOrDateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => (day - rhs).into(),
            DateOrDateTime::DateTime(dt) => (dt - rhs).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    #[test]
    fn inc_month_simple() {
        let date: DateOrDateTime = Utc
            .from_local_date(
                &NaiveDate::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap()
            .into();

        let date_time: DateOrDateTime = Utc
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap()
            .into();

        let next = date.inc_month(1);
        assert_eq!(date.year(), next.year());
        assert_eq!(date.month() + 1, next.month());

        let next = date_time.inc_month(12);
        assert_eq!(date_time.year() + 1, next.year());
        assert_eq!(date_time.month(), next.month());
    }

    #[test]
    fn next_weekday() {
        let date: DateOrDateTime = Utc
            .from_local_date(
                &NaiveDate::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(), //SAT
            )
            .unwrap()
            .into();

        assert_eq!(date + Duration::days(6), date.next_weekday(Weekday::Fri));
        assert_eq!(date + Duration::days(1), date.next_weekday(Weekday::Sun));
        assert_eq!(date + Duration::days(7), date.next_weekday(Weekday::Sat));
    }

    #[test]
    fn next_weekdays() {
        let date: DateOrDateTime = Utc
            .from_local_date(
                &NaiveDate::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(), //SAT
            )
            .unwrap()
            .into();

        assert_eq!(
            date + Duration::days(1),
            date.next_weekdays(&[Weekday::Fri, Weekday::Sun])
        );
        assert_eq!(
            date + Duration::days(6),
            date.next_weekdays(&[Weekday::Fri])
        );
        assert_eq!(
            date + Duration::days(6),
            date.next_weekdays(&[Weekday::Fri, Weekday::Sat])
        );
        assert_eq!(
            date + Duration::days(2),
            date.next_weekdays(&[Weekday::Mon, Weekday::Fri])
        );
    }

    #[test]
    fn move_by_day() {
        let date: DateOrDateTime = Utc
            .from_local_date(
                &NaiveDate::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap()
            .into();

        let first_sunday = date.move_by_delta(&Delta::new(1, Weekday::Sun));
        assert_eq!(first_sunday.day(), 6);

        let last_sunday = date.move_by_delta(&Delta::new(-1, Weekday::Sun));
        assert_eq!(last_sunday.day(), 27);
    }

    #[test]
    fn check_intersects_date() {
        let e: DateOrDateTime = Date::<Utc>::from_utc(NaiveDate::from_ymd(2022, 2, 10), Utc).into();

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::FinishesPast
        );

        let dt_start = DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );

        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        // Date instead of DateTime
        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );
    }

    #[test]
    fn check_intersects_date_time() {
        let e: DateOrDateTime = Date::<Utc>::from_utc(NaiveDate::from_ymd(2022, 2, 10), Utc)
            .and_hms(8, 0, 0)
            .into();

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::FinishesPast
        );

        let dt_start = DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsFuture
        );

        let dt_start = DateTime::parse_from_str("20220210T023000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        let dt_end = DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        // Date instead of DateTime
        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        let dt_end = DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc)
            .date()
            .into();
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );
    }
}
