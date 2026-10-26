use chrono::{Datelike, LocalResult, TimeZone, Timelike, Utc};
use std::{cmp::Ordering, ops::Range};

use crate::{
    VEvent,
    by_day::ByDay,
    date_or_date_time::DateOrDateTime,
    rrule::{Options, RRule},
};
use chrono::Duration;

#[derive(Debug, Clone)]
pub struct VEventIterator<'a> {
    event: &'a VEvent,
    last_occurrence: Option<DateOrDateTime>,
    count: u32,
}

impl<'a> VEventIterator<'a> {
    pub(crate) fn new(event: &'a VEvent) -> Self {
        Self {
            event,
            last_occurrence: None,
            count: 0,
        }
    }

    fn to_wall_clock(&self, dt: DateOrDateTime) -> DateOrDateTime {
        match (dt, self.event.dt_start_tz) {
            (DateOrDateTime::DateTime(utc_dt), Some(tz)) => {
                let local = utc_dt.with_timezone(&tz).naive_local();
                DateOrDateTime::DateTime(Utc.from_utc_datetime(&local))
            }
            _ => dt,
        }
    }

    fn from_wall_clock(&self, dt: DateOrDateTime) -> DateOrDateTime {
        match (dt, self.event.dt_start_tz) {
            (DateOrDateTime::DateTime(wall_dt), Some(tz)) => {
                let naive = wall_dt.naive_utc();
                let utc_dt = tz
                    .from_local_datetime(&naive)
                    .earliest()
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or(wall_dt);
                DateOrDateTime::DateTime(utc_dt)
            }
            _ => dt,
        }
    }

    fn with_year_month_clamped_day(
        base: DateOrDateTime,
        year: i32,
        month: u32,
        month_day: i8,
        default_day: u32,
    ) -> DateOrDateTime {
        let mut max_day = 31;
        while matches!(
            Utc.with_ymd_and_hms(year, month, max_day, 0, 0, 0),
            LocalResult::None
        ) && max_day > 1
        {
            max_day -= 1;
        }
        let d = if month_day == 0 {
            default_day.min(max_day)
        } else if month_day > 0 {
            (month_day as u32).min(max_day)
        } else {
            (max_day as i32 + 1 + month_day as i32).max(1) as u32
        };
        let target_date = Utc.with_ymd_and_hms(year, month, d, 0, 0, 0).unwrap();
        match base {
            DateOrDateTime::WholeDay(_) => DateOrDateTime::WholeDay(target_date),
            DateOrDateTime::DateTime(dt) => DateOrDateTime::DateTime(
                Utc.with_ymd_and_hms(
                    target_date.year(),
                    target_date.month(),
                    target_date.day(),
                    dt.hour(),
                    dt.minute(),
                    dt.second(),
                )
                .unwrap(),
            ),
        }
    }

    fn get_next_occurrence_according_to_rule(
        &mut self,
        last_occurrence: DateOrDateTime,
        rrule: &RRule,
    ) -> Option<DateOrDateTime> {
        let last_occurrence = self.to_wall_clock(last_occurrence);
        let dt_start_wall = self.to_wall_clock(self.event.dt_start);
        let wall_next = match rrule {
            RRule::Yearly(_rrule) => Self::with_year_month_clamped_day(
                last_occurrence,
                last_occurrence.year() + 1,
                dt_start_wall.month(),
                0,
                dt_start_wall.day(),
            ),

            RRule::YearlyByMonthByDay(rrule) => {
                let next_occurrence = last_occurrence.next_by_day(&rrule.day);
                if next_occurrence.year() == last_occurrence.year()
                    && next_occurrence.month() == rrule.month as u32
                    && next_occurrence > last_occurrence
                {
                    next_occurrence
                } else {
                    let next_year = last_occurrence.year()
                        + rrule.common_options().interval.unwrap_or(1) as i32;
                    let target_month_start = last_occurrence
                        .substitute(
                            Some(next_year),
                            Some(rrule.month as u32),
                            Some(1),
                            None,
                            None,
                            None,
                        )
                        .unwrap();

                    let mut next_occurrence = target_month_start;
                    match &rrule.day {
                        ByDay::Simple(weekdays) => {
                            if !weekdays.contains(&target_month_start.date().weekday()) {
                                next_occurrence = target_month_start.next_by_day(&rrule.day);
                            }
                        }
                        ByDay::Delta(_) => {
                            next_occurrence = target_month_start.next_by_day(&rrule.day);
                        }
                    }

                    next_occurrence
                }
            }

            RRule::YearlyByMonthByMonthDay(rrule) => Self::with_year_month_clamped_day(
                last_occurrence,
                last_occurrence.year() + 1,
                rrule.month as u32,
                rrule.month_day,
                dt_start_wall.day(),
            ),

            RRule::MonthlyByMonthDay(rrule) => {
                let mut year = last_occurrence.year();
                let mut month = last_occurrence.month() + 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
                Self::with_year_month_clamped_day(
                    last_occurrence,
                    year,
                    month,
                    rrule.month_day,
                    dt_start_wall.day(),
                )
            }

            RRule::MonthlyByDay(rrule) => {
                let interval = rrule.common_options().interval.unwrap_or(1);
                match &rrule.day {
                    ByDay::Simple(weekdays) => {
                        let candidate = last_occurrence.next_by_day(&rrule.day);
                        if candidate.year() == last_occurrence.year()
                            && candidate.month() == last_occurrence.month()
                        {
                            candidate
                        } else {
                            let target_month_start = last_occurrence
                                .substitute(None, None, Some(1), None, None, None)
                                .unwrap()
                                .inc_month(interval);
                            if weekdays.contains(&target_month_start.date().weekday()) {
                                target_month_start
                            } else {
                                target_month_start.next_by_day(&rrule.day)
                            }
                        }
                    }
                    ByDay::Delta(_) => {
                        let next_month = last_occurrence
                            .substitute(None, None, Some(1), None, None, None)
                            .unwrap()
                            .inc_month(interval);
                        next_month.next_by_day(&rrule.day)
                    }
                }
            }

            RRule::Weekly(_rrule) => last_occurrence + Duration::days(7),

            RRule::WeeklyByDay(rrule) => {
                let interval = rrule.common_options().interval.unwrap_or(1) as i64;
                let mut next_occurrence = last_occurrence.next_by_day(&rrule.day);
                let dt_start_wall = self.to_wall_clock(self.event.dt_start);

                loop {
                    let dt_start_monday = dt_start_wall.date()
                        - Duration::days(
                            dt_start_wall.date().weekday().num_days_from_monday() as i64
                        );
                    let next_monday = next_occurrence.date()
                        - Duration::days(
                            next_occurrence.date().weekday().num_days_from_monday() as i64
                        );
                    let diff_days = (next_monday - dt_start_monday).num_days();
                    let diff_weeks = diff_days / 7;

                    if diff_weeks % interval == 0 {
                        break;
                    }
                    next_occurrence = next_occurrence.next_by_day(&rrule.day);
                }

                next_occurrence
            }

            RRule::Daily(_rrule) => last_occurrence + Duration::days(1),
        };

        let next_occurrence = self.from_wall_clock(wall_next);
        if !rrule.is_expired(next_occurrence) {
            self.last_occurrence = Some(next_occurrence);
            self.last_occurrence
        } else {
            None
        }
    }

    fn get_next_occurrence_according_to_rule_and_iterations(&mut self) -> Option<DateOrDateTime> {
        if let Some(last_occurrence) = self.last_occurrence {
            self.event.rrule.as_ref().and_then(|rrule| {
                if rrule.is_out_of_count(self.count) {
                    return None;
                }
                let mut next_occurrence = Some(last_occurrence);
                let mut iterations = match rrule {
                    RRule::WeeklyByDay(_)
                    | RRule::YearlyByMonthByDay(_)
                    | RRule::MonthlyByDay(_) => 1,
                    _ => rrule.common_options().interval.unwrap_or(1),
                };
                while iterations > 0 && next_occurrence.is_some() {
                    next_occurrence =
                        self.get_next_occurrence_according_to_rule(next_occurrence.unwrap(), rrule);
                    iterations -= 1;
                }

                next_occurrence
            })
        } else {
            if let Some(rrule) = &self.event.rrule {
                if rrule.is_out_of_count(self.count) || rrule.is_expired(self.event.dt_start) {
                    return None;
                }
            }
            self.last_occurrence = Some(self.event.dt_start);
            Some(self.event.dt_start)
        }
    }
}

impl<'a> Iterator for VEventIterator<'a> {
    type Item = Range<DateOrDateTime>;

    fn next(&mut self) -> Option<Self::Item> {
        log::trace!("function next({:?}) called", self);

        let mut next = self.get_next_occurrence_according_to_rule_and_iterations();
        log::trace!("next == {:?}", next);

        loop {
            // remove dates appearing in ExDate field
            if let Some(next_non_empty) = next {
                log::trace!("next_non_empty == {:?}", next_non_empty);

                if !self.event.exdates.iter().any(|exdate| {
                    // we check only for date comparison and not time because of the weird handling
                    // of timezones in EXDATE. This should be enough since the repetition can be at
                    // most per day.
                    next_non_empty
                        .date()
                        .date_naive()
                        .cmp(&exdate.date_time.date().date_naive())
                        == Ordering::Equal
                }) {
                    // keep count
                    self.count += 1;

                    // calculate how long it's supposed to last
                    let delta = self.event.dt_end - self.event.dt_start;
                    let next_non_empty_end = next_non_empty + delta;
                    return Some(Range {
                        start: next_non_empty,
                        end: next_non_empty_end,
                    });
                } else {
                    next = self.get_next_occurrence_according_to_rule_and_iterations();
                }
            } else {
                return None;
            }
        }
    }
}
