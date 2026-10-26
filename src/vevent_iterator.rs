use chrono::{Datelike, Duration, NaiveDate, TimeZone, Timelike, Utc};
use std::{cmp::Ordering, ops::Range};

use crate::{VEvent, date_or_date_time::DateOrDateTime, frequency::Frequency, rrule::RRule};

#[derive(Debug, Clone)]
pub struct VEventIterator<'a> {
    event: &'a VEvent,
    last_occurrence: Option<DateOrDateTime>,
    count: u32,
    rdate_idx: usize,
    peeked_rrule: Option<DateOrDateTime>,
    rrule_exhausted: bool,
}

impl<'a> VEventIterator<'a> {
    pub(crate) fn new(event: &'a VEvent) -> Self {
        Self {
            event,
            last_occurrence: None,
            count: 0,
            rdate_idx: 0,
            peeked_rrule: None,
            rrule_exhausted: false,
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

    fn apply_by_set_pos(
        candidates: Vec<DateOrDateTime>,
        by_set_pos: &[i16],
    ) -> Vec<DateOrDateTime> {
        if by_set_pos.is_empty() {
            return candidates;
        }
        let len = candidates.len() as i32;
        let mut result = Vec::new();
        for &pos in by_set_pos {
            let idx = if pos > 0 {
                pos as i32 - 1
            } else {
                len + pos as i32
            };
            if idx >= 0 && idx < len {
                result.push(candidates[idx as usize]);
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    fn candidates_in_month(
        base: DateOrDateTime,
        year: i32,
        month: u32,
        rrule: &RRule,
        default_day: u32,
    ) -> Vec<DateOrDateTime> {
        let month_day_candidates: Option<Vec<DateOrDateTime>> = if rrule.by_month_day.is_empty() {
            None
        } else {
            let mut v: Vec<DateOrDateTime> = rrule
                .by_month_day
                .iter()
                .map(|&md| Self::with_year_month_clamped_day(base, year, month, md, default_day))
                .collect();
            v.sort_unstable();
            v.dedup();
            Some(v)
        };

        let by_day_candidates: Option<Vec<DateOrDateTime>> = if rrule.by_day.is_empty() {
            None
        } else {
            Some(base.matching_days_in_month(year, month, &rrule.by_day))
        };

        match (month_day_candidates, by_day_candidates) {
            (Some(md), Some(bd)) => md.into_iter().filter(|d| bd.contains(d)).collect(),
            (Some(md), None) => md,
            (None, Some(bd)) => bd,
            (None, None) => vec![Self::with_year_month_clamped_day(
                base,
                year,
                month,
                0,
                default_day,
            )],
        }
    }

    fn candidates_in_year(
        base: DateOrDateTime,
        year: i32,
        rrule: &RRule,
        default_month: u32,
        default_day: u32,
    ) -> Vec<DateOrDateTime> {
        let mut months: Vec<u32> = if rrule.by_month.is_empty() {
            vec![default_month]
        } else {
            rrule.by_month.iter().map(|&m| m as u32).collect()
        };
        months.sort_unstable();
        months.dedup();

        let mut all = Vec::new();
        for m in months {
            all.extend(Self::candidates_in_month(base, year, m, rrule, default_day));
        }
        all.sort_unstable();
        all.dedup();
        Self::apply_by_set_pos(all, &rrule.by_set_pos)
    }

    pub(crate) fn next_rrule_occurrence(
        dt_start: DateOrDateTime,
        last_occurrence: DateOrDateTime,
        rrule: &RRule,
    ) -> Option<DateOrDateTime> {
        let last_occurrence = Self::to_wall_clock(last_occurrence);
        let dt_start_wall = Self::to_wall_clock(dt_start);
        let interval = rrule.interval.max(1);

        let wall_next = match rrule.frequency {
            Frequency::Yearly => {
                let current_year_candidates = Self::candidates_in_year(
                    last_occurrence,
                    last_occurrence.year(),
                    rrule,
                    dt_start_wall.month(),
                    dt_start_wall.day(),
                );
                if let Some(&next_in_year) = current_year_candidates
                    .iter()
                    .find(|&&d| d > last_occurrence)
                {
                    next_in_year
                } else {
                    let mut year = last_occurrence.year() + interval as i32;
                    loop {
                        let year_candidates = Self::candidates_in_year(
                            last_occurrence,
                            year,
                            rrule,
                            dt_start_wall.month(),
                            dt_start_wall.day(),
                        );
                        if let Some(&first) = year_candidates.first() {
                            break first;
                        }
                        year += interval as i32;
                    }
                }
            }

            Frequency::Monthly => {
                let current_month_ok = rrule.by_month.is_empty()
                    || rrule.by_month.contains(&(last_occurrence.month() as u8));
                let current_month_candidates = if current_month_ok {
                    Self::apply_by_set_pos(
                        Self::candidates_in_month(
                            last_occurrence,
                            last_occurrence.year(),
                            last_occurrence.month(),
                            rrule,
                            dt_start_wall.day(),
                        ),
                        &rrule.by_set_pos,
                    )
                } else {
                    Vec::new()
                };

                if let Some(&next_in_month) = current_month_candidates
                    .iter()
                    .find(|&&d| d > last_occurrence)
                {
                    next_in_month
                } else {
                    let mut step_months = interval;
                    loop {
                        let total_months = (last_occurrence.month() - 1) + step_months;
                        let year = last_occurrence.year() + (total_months / 12) as i32;
                        let month = (total_months % 12) + 1;
                        if rrule.by_month.is_empty() || rrule.by_month.contains(&(month as u8)) {
                            let month_candidates = Self::apply_by_set_pos(
                                Self::candidates_in_month(
                                    last_occurrence,
                                    year,
                                    month,
                                    rrule,
                                    dt_start_wall.day(),
                                ),
                                &rrule.by_set_pos,
                            );
                            if let Some(&first) = month_candidates.first() {
                                break first;
                            }
                        }
                        step_months += interval;
                    }
                }
            }

            Frequency::Weekly => {
                if !rrule.by_day.is_empty() {
                    let interval_i64 = interval as i64;
                    let wkst = rrule.wkst.unwrap_or(chrono::Weekday::Mon);
                    let days_since_wkst = |wd: chrono::Weekday| -> i64 {
                        ((wd.num_days_from_monday() + 7 - wkst.num_days_from_monday()) % 7) as i64
                    };
                    let dt_start_week_start = dt_start_wall.date_naive()
                        - Duration::days(days_since_wkst(dt_start_wall.date_naive().weekday()));
                    let mut next_occurrence = last_occurrence.next_by_day(&rrule.by_day);

                    loop {
                        let next_week_start = next_occurrence.date_naive()
                            - Duration::days(days_since_wkst(
                                next_occurrence.date_naive().weekday(),
                            ));
                        let diff_days = (next_week_start - dt_start_week_start).num_days();
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
            Some(next_occurrence)
        } else {
            None
        }
    }

    fn get_next_occurrence_according_to_rule(
        &mut self,
        last_occurrence: DateOrDateTime,
        rrule: &RRule,
    ) -> Option<DateOrDateTime> {
        let next_occurrence =
            Self::next_rrule_occurrence(self.event.dt_start, last_occurrence, rrule)?;
        self.last_occurrence = Some(next_occurrence);
        self.last_occurrence
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
            if let Some(rrule) = &self.event.rrule
                && (rrule.is_out_of_count(self.count) || rrule.is_expired(self.event.dt_start))
            {
                return None;
            }
            self.last_occurrence = Some(self.event.dt_start);
            Some(self.event.dt_start)
        }
    }

    fn is_excluded(&self, dt: DateOrDateTime) -> bool {
        self.event
            .exdates
            .iter()
            .any(|exdate| dt.date_naive() == exdate.date_naive())
    }

    fn is_overridden(&self, dt: DateOrDateTime) -> bool {
        self.event
            .overridden_dates
            .iter()
            .any(|ov| dt.date_naive() == ov.date_naive())
    }

    fn next_valid_rrule_occurrence(&mut self) -> Option<DateOrDateTime> {
        if self.rrule_exhausted {
            return None;
        }
        loop {
            match self.get_next_occurrence_according_to_rule_and_iterations() {
                Some(candidate) => {
                    if !self.is_excluded(candidate) {
                        self.count += 1;
                        if !self.is_overridden(candidate) {
                            return Some(candidate);
                        }
                    }
                }
                None => {
                    self.rrule_exhausted = true;
                    return None;
                }
            }
        }
    }

    fn next_valid_rdate(&mut self) -> Option<DateOrDateTime> {
        while let Some(&rdate) = self.event.rdates.get(self.rdate_idx) {
            if self.is_excluded(rdate) || self.is_overridden(rdate) {
                self.rdate_idx += 1;
            } else {
                return Some(rdate);
            }
        }
        None
    }
}

impl Iterator for VEventIterator<'_> {
    type Item = Range<DateOrDateTime>;

    fn next(&mut self) -> Option<Self::Item> {
        log::trace!("function next({:?}) called", self);

        if self.peeked_rrule.is_none() && !self.rrule_exhausted {
            self.peeked_rrule = self.next_valid_rrule_occurrence();
        }

        let rdate_candidate = self.next_valid_rdate();

        let next_start = match (self.peeked_rrule, rdate_candidate) {
            (Some(rrule_dt), Some(rdate_dt)) => match rrule_dt.cmp(&rdate_dt) {
                Ordering::Less => self.peeked_rrule.take(),
                Ordering::Greater => {
                    self.rdate_idx += 1;
                    Some(rdate_dt)
                }
                Ordering::Equal => {
                    self.rdate_idx += 1;
                    self.peeked_rrule.take()
                }
            },
            (Some(_), None) => self.peeked_rrule.take(),
            (None, Some(rdate_dt)) => {
                self.rdate_idx += 1;
                Some(rdate_dt)
            }
            (None, None) => None,
        }?;

        let delta = self.event.dt_end - self.event.dt_start;
        let next_end = next_start + delta;
        Some(Range {
            start: next_start,
            end: next_end,
        })
    }
}
