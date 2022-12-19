use crate::by_day::{ByDay, Delta};
use chrono::{DateTime, Datelike, Duration, LocalResult, TimeZone, Timelike, Utc, Weekday};
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
    WholeDay(DateTime<Utc>),
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
        let date = Utc
            .with_ymd_and_hms(
                year.unwrap_or_else(|| self.year()),
                month.unwrap_or_else(|| self.month()),
                day.unwrap_or_else(|| self.day()),
                0,
                0,
                0,
            )
            .unwrap();

        Ok(match self {
            DateOrDateTime::WholeDay(_) => {
                if hour.is_some() || minute.is_some() || second.is_some() {
                    return Err(SubstitutionError::ConstructingDateTimeBySubstitutingWholeDay);
                }
                DateOrDateTime::WholeDay(date)
            }
            DateOrDateTime::DateTime(dt) => DateOrDateTime::DateTime(
                Utc.with_ymd_and_hms(
                    date.year(),
                    date.month(),
                    date.day(),
                    hour.unwrap_or_else(|| dt.hour()),
                    minute.unwrap_or_else(|| dt.minute()),
                    second.unwrap_or_else(|| dt.second()),
                )
                .unwrap(),
            ),
        })
    }

    pub fn substitute_time_with(self, time: impl Into<DateOrDateTime>) -> Self {
        match time.into() {
            DateOrDateTime::WholeDay(_) => self,
            DateOrDateTime::DateTime(dt) => DateOrDateTime::DateTime(
                Utc.with_ymd_and_hms(
                    dt.year(),
                    dt.month(),
                    dt.day(),
                    dt.hour(),
                    dt.minute(),
                    dt.second(),
                )
                .unwrap(),
            ),
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

    pub fn equals_date(self, date: DateTime<Utc>) -> bool {
        match self {
            DateOrDateTime::WholeDay(d) => date == d,
            DateOrDateTime::DateTime(dt) => date == dt,
        }
    }

    pub fn equals_date_time(self, date_time: DateTime<Utc>) -> bool {
        match self {
            DateOrDateTime::WholeDay(d) => d == date_time,
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
            let mut date = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0);
            while matches!(date, LocalResult::None) {
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }

                date = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0);
            }
            date.unwrap()
        };

        match self {
            DateOrDateTime::WholeDay(_) => DateOrDateTime::WholeDay(date),
            DateOrDateTime::DateTime(dt) => DateOrDateTime::DateTime(
                Utc.with_ymd_and_hms(
                    date.year(),
                    date.month(),
                    date.day(),
                    dt.hour(),
                    dt.minute(),
                    dt.second(),
                )
                .unwrap(),
            ),
        }
    }

    pub fn inc_year(&self, increment: u32) -> DateOrDateTime {
        match self {
            DateOrDateTime::WholeDay(d) => {
                let d = Utc
                    .with_ymd_and_hms(d.year() + increment as i32, d.month(), d.day(), 0, 0, 0)
                    .unwrap();
                Self::WholeDay(d)
            }
            DateOrDateTime::DateTime(d) => {
                let d = Utc
                    .with_ymd_and_hms(
                        d.year() + increment as i32,
                        d.month(),
                        d.day(),
                        d.hour(),
                        d.minute(),
                        d.second(),
                    )
                    .unwrap();
                Self::DateTime(d)
            }
        }
    }

    pub fn date(self) -> DateTime<Utc> {
        match self {
            DateOrDateTime::WholeDay(d) => d,
            DateOrDateTime::DateTime(dt) => dt,
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
        log::trace!("intersects({self:?}, dt_start == {dt_start:?}, dt_end == {dt_end:?})");

        match self {
            DateOrDateTime::WholeDay(day) => {
                let d_start = Utc
                    .with_ymd_and_hms(dt_start.year(), dt_start.month(), dt_start.day(), 0, 0, 0)
                    .unwrap();
                let d_end = Utc
                    .with_ymd_and_hms(dt_end.year(), dt_end.month(), dt_end.day(), 0, 0, 0)
                    .unwrap();

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
                    DateOrDateTime::WholeDay(d) => Utc
                        .with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                        .unwrap(),
                };
                let dt_end = match dt_end {
                    DateOrDateTime::DateTime(dt) => dt,
                    DateOrDateTime::WholeDay(d) => Utc
                        .with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                        .unwrap(),
                };

                match (
                    dt_start.date_naive().cmp(&dt.date_naive()),
                    dt_end.date_naive().cmp(&dt.date_naive()),
                ) {
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

impl DateOrDateTime {
    pub fn succ_day(&self) -> DateOrDateTime {
        match self {
            DateOrDateTime::WholeDay(whole) => {
                DateOrDateTime::WholeDay(*whole + chrono::Duration::days(1))
            }
            DateOrDateTime::DateTime(dt) => {
                DateOrDateTime::DateTime(*dt + chrono::Duration::days(1))
            }
        }
    }

    pub fn as_datetime(&self) -> DateTime<Utc> {
        match self {
            DateOrDateTime::WholeDay(day) => *day,
            DateOrDateTime::DateTime(dt) => *dt,
        }
    }
}

impl PartialOrd for DateOrDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // convert in date time if necessary
        let self_dt = match self {
            DateOrDateTime::DateTime(dt) => *dt,
            DateOrDateTime::WholeDay(dt) => *dt,
        };

        let other_dt = match other {
            DateOrDateTime::DateTime(dt) => *dt,
            DateOrDateTime::WholeDay(dt) => *dt,
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
            DateOrDateTime::WholeDay(d) => d,
            DateOrDateTime::DateTime(dt) => dt,
        };

        let dt_rhs = match rhs {
            DateOrDateTime::WholeDay(d) => d,
            DateOrDateTime::DateTime(dt) => dt,
        };

        dt_self - dt_rhs
    }
}

impl Add<Duration> for DateOrDateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => Self::WholeDay(day + rhs),
            DateOrDateTime::DateTime(dt) => Self::DateTime(dt + rhs),
        }
    }
}

impl Sub<Duration> for DateOrDateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        match self {
            DateOrDateTime::WholeDay(day) => Self::WholeDay(day - rhs),
            DateOrDateTime::DateTime(dt) => Self::DateTime(dt - rhs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    #[test]
    fn inc_month_simple() {
        let date: DateOrDateTime = DateOrDateTime::WholeDay(
            Utc.from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap(),
        );

        let date_time: DateOrDateTime = DateOrDateTime::WholeDay(
            Utc.from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap(),
        );

        let next = date.inc_month(1);
        assert_eq!(date.year(), next.year());
        assert_eq!(date.month() + 1, next.month());

        let next = date_time.inc_month(12);
        assert_eq!(date_time.year() + 1, next.year());
        assert_eq!(date_time.month(), next.month());
    }

    #[test]
    fn next_weekday() {
        let date: DateOrDateTime = DateOrDateTime::WholeDay(
            Utc.from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(), //SAT
            )
            .unwrap(),
        );

        assert_eq!(date + Duration::days(6), date.next_weekday(Weekday::Fri));
        assert_eq!(date + Duration::days(1), date.next_weekday(Weekday::Sun));
        assert_eq!(date + Duration::days(7), date.next_weekday(Weekday::Sat));
    }

    #[test]
    fn next_weekdays() {
        let date: DateOrDateTime = DateOrDateTime::WholeDay(
            Utc.from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(), //SAT
            )
            .unwrap(),
        );

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
        let date: DateOrDateTime = DateOrDateTime::WholeDay(
            Utc.from_local_datetime(
                &NaiveDateTime::parse_from_str("20220205T000000", "%Y%m%dT%H%M%S").unwrap(),
            )
            .unwrap(),
        );

        let first_sunday = date.move_by_delta(&Delta::new(1, Weekday::Sun));
        assert_eq!(first_sunday.day(), 6);

        let last_sunday = date.move_by_delta(&Delta::new(-1, Weekday::Sun));
        assert_eq!(last_sunday.day(), 27);
    }

    #[test]
    fn check_intersects_date() {
        let e: DateOrDateTime =
            DateOrDateTime::WholeDay(Utc.with_ymd_and_hms(2022, 2, 10, 0, 0, 0).unwrap());

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::FinishesPast
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsFuture
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        // Date instead of DateTime
        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );
    }

    #[test]
    fn check_intersects_date_time() {
        let e: DateOrDateTime =
            DateOrDateTime::DateTime(Utc.with_ymd_and_hms(2022, 2, 10, 8, 0, 0).unwrap());

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::FinishesPast
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsFuture
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsFuture
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );

        let dt_start = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20220210T023000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::DateTime(
            DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        // Date instead of DateTime
        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsSameDayEndsFuture
        );

        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartsPastEndsSameDay
        );

        let dt_start = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        let dt_end = DateOrDateTime::WholeDay(
            DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(
            e.intersects(dt_start, dt_end).unwrap(),
            EventOverlap::StartSameDayEndsSameDay
        );
    }
}
