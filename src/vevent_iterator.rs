use chrono::{Datelike, Duration, NaiveDate, TimeZone, Timelike, Utc};
use std::ops::Range;

use crate::{VEvent, date_or_date_time::DateOrDateTime, frequency::Frequency, rrule::RRule};

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

    fn to_wall_clock(dt: DateOrDateTime) -> DateOrDateTime {
        match dt {
            DateOrDateTime::DateTime { utc, tz: Some(tz) } => {
                let local = utc.with_timezone(&tz).naive_local();
                DateOrDateTime::DateTime {
                    utc: Utc.from_utc_datetime(&local),
                    tz: Some(tz),
                }
            }
            _ => dt,
        }
    }

    fn from_wall_clock(dt: DateOrDateTime) -> DateOrDateTime {
        match dt {
            DateOrDateTime::DateTime {
                utc: wall_dt,
                tz: Some(tz),
            } => {
                let naive = wall_dt.naive_utc();
                let utc_dt = tz
                    .from_local_datetime(&naive)
                    .earliest()
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or(wall_dt);
                DateOrDateTime::DateTime {
                    utc: utc_dt,
                    tz: Some(tz),
                }
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
        while NaiveDate::from_ymd_opt(year, month, max_day).is_none() && max_day > 1 {
            max_day -= 1;
        }
        let d = if month_day == 0 {
            default_day.min(max_day)
        } else if month_day > 0 {
            (month_day as u32).min(max_day)
        } else {
            (max_day as i32 + 1 + month_day as i32).max(1) as u32
        };
        let target_date = NaiveDate::from_ymd_opt(year, month, d).unwrap();
        match base {
            DateOrDateTime::WholeDay(_) => DateOrDateTime::WholeDay(target_date),
            DateOrDateTime::DateTime { utc, tz } => DateOrDateTime::DateTime {
                utc: target_date
                    .and_hms_opt(utc.hour(), utc.minute(), utc.second())
                    .unwrap()
                    .and_utc(),
                tz,
            },
        }
    }

    fn get_next_occurrence_according_to_rule(
        &mut self,
        last_occurrence: DateOrDateTime,
        rrule: &RRule,
    ) -> Option<DateOrDateTime> {
        let last_occurrence = Self::to_wall_clock(last_occurrence);
        let dt_start_wall = Self::to_wall_clock(self.event.dt_start);
        let interval = rrule.interval.max(1);

        let wall_next = match rrule.frequency {
            Frequency::Yearly => {
                let target_month = rrule
                    .by_month
                    .map(|m| m as u32)
                    .unwrap_or_else(|| dt_start_wall.month());

                if !rrule.by_day.is_empty() {
                    if last_occurrence.month() == target_month {
                        if let Some(next_in_year) = last_occurrence
                            .matching_days_in_month(
                                last_occurrence.year(),
                                target_month,
                                &rrule.by_day,
                            )
                            .into_iter()
                            .find(|&d| d > last_occurrence)
                        {
                            next_in_year
                        } else {
                            let mut year = last_occurrence.year() + interval as i32;
                            loop {
                                if let Some(first) = last_occurrence
                                    .matching_days_in_month(year, target_month, &rrule.by_day)
                                    .into_iter()
                                    .next()
                                {
                                    break first;
                                }
                                year += interval as i32;
                            }
                        }
                    } else {
                        let mut year = if last_occurrence.month() < target_month {
                            last_occurrence.year()
                        } else {
                            last_occurrence.year() + interval as i32
                        };
                        loop {
                            if let Some(first) = last_occurrence
                                .matching_days_in_month(year, target_month, &rrule.by_day)
                                .into_iter()
                                .next()
                            {
                                break first;
                            }
                            year += interval as i32;
                        }
                    }
                } else {
                    Self::with_year_month_clamped_day(
                        last_occurrence,
                        last_occurrence.year() + interval as i32,
                        target_month,
                        rrule.by_month_day.unwrap_or(0),
                        dt_start_wall.day(),
                    )
                }
            }

            Frequency::Monthly => {
                if !rrule.by_day.is_empty() {
                    if let Some(next_in_month) = last_occurrence
                        .matching_days_in_month(
                            last_occurrence.year(),
                            last_occurrence.month(),
                            &rrule.by_day,
                        )
                        .into_iter()
                        .find(|&d| d > last_occurrence)
                    {
                        next_in_month
                    } else {
                        let mut step_months = interval;
                        loop {
                            let total_months = (last_occurrence.month() - 1) + step_months;
                            let year = last_occurrence.year() + (total_months / 12) as i32;
                            let month = (total_months % 12) + 1;
                            if let Some(first) = last_occurrence
                                .matching_days_in_month(year, month, &rrule.by_day)
                                .into_iter()
                                .next()
                            {
                                break first;
                            }
                            step_months += interval;
                        }
                    }
                } else {
                    let total_months = (last_occurrence.month() - 1) + interval;
                    let year = last_occurrence.year() + (total_months / 12) as i32;
                    let month = (total_months % 12) + 1;
                    Self::with_year_month_clamped_day(
                        last_occurrence,
                        year,
                        month,
                        rrule.by_month_day.unwrap_or(0),
                        dt_start_wall.day(),
                    )
                }
            }

            Frequency::Weekly => {
                if !rrule.by_day.is_empty() {
                    let interval_i64 = interval as i64;
                    let mut next_occurrence = last_occurrence.next_by_day(&rrule.by_day);

                    loop {
                        let dt_start_monday = dt_start_wall.date_naive()
                            - Duration::days(
                                dt_start_wall.date_naive().weekday().num_days_from_monday() as i64,
                            );
                        let next_monday = next_occurrence.date_naive()
                            - Duration::days(
                                next_occurrence
                                    .date_naive()
                                    .weekday()
                                    .num_days_from_monday() as i64,
                            );
                        let diff_days = (next_monday - dt_start_monday).num_days();
                        let diff_weeks = diff_days / 7;

                        if diff_weeks % interval_i64 == 0 {
                            break;
                        }
                        next_occurrence = next_occurrence.next_by_day(&rrule.by_day);
                    }

                    next_occurrence
                } else {
                    last_occurrence + Duration::days(7 * interval as i64)
                }
            }

            Frequency::Daily => last_occurrence + Duration::days(interval as i64),
        };

        let next_occurrence = Self::from_wall_clock(wall_next);
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
                self.get_next_occurrence_according_to_rule(last_occurrence, rrule)
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

                if !self
                    .event
                    .exdates
                    .iter()
                    .any(|exdate| next_non_empty.date_naive() == exdate.date_naive())
                {
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
