use std::{cmp::Ordering, ops::Range};

use crate::{
    date_or_date_time::DateOrDateTime,
    rrule::{Options, RRule},
    VEvent,
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

    fn get_next_occurrence_according_to_rule(
        &mut self,
        last_occurrence: DateOrDateTime,
        rrule: &RRule,
    ) -> Option<DateOrDateTime> {
        match rrule {
            RRule::Yearly(_rrule) => {
                let next_occurrence = last_occurrence.inc_year(1);
                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::YearlyByMonthByDay(_rrule) => {
                unimplemented!();
            }

            RRule::YearlyByMonthByMonthDay(_rrule) => {
                let next_occurrence = last_occurrence.inc_year(1);
                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::MonthlyByMonthDay(rrule) => {
                let next_occurrence =
                    last_occurrence.inc_month(rrule.common_options().interval.unwrap_or(1));

                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::MonthlyByDay(rrule) => {
                let next_month = last_occurrence
                    .substitute(
                        Some(if last_occurrence.month() == 12 {
                            last_occurrence.year() + 1
                        } else {
                            last_occurrence.year()
                        }),
                        Some(if last_occurrence.month() == 12 {
                            1
                        } else {
                            last_occurrence.month() + 1
                        }),
                        Some(1),
                        None,
                        None,
                        None,
                    )
                    .unwrap();

                // Calculate 1SU or -1SU... done in DateOrDatetime
                let next_occurrence = next_month.next_by_day(&rrule.day);

                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::Weekly(rrule) => {
                let next_occurrence = last_occurrence + Duration::days(7);

                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::WeeklyByDay(rrule) => {
                let next_occurrence = last_occurrence.next_by_day(&rrule.day);
                log::debug!(
                    "last_occurrence == {:?}, next_occurrence == {:?}",
                    last_occurrence,
                    next_occurrence
                );

                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }

            RRule::Daily(rrule) => {
                let next_occurrence = last_occurrence + Duration::days(1);

                if !rrule.is_expired(next_occurrence) {
                    self.last_occurrence = Some(next_occurrence);
                    self.last_occurrence
                } else {
                    None
                }
            }
        }
    }

    fn get_next_occurrence_according_to_rule_and_iterations(&mut self) -> Option<DateOrDateTime> {
        if let Some(last_occurrence) = self.last_occurrence {
            self.event.rrule.as_ref().and_then(|rrule| {
                if rrule.is_out_of_count(self.count) {
                    return None;
                }
                let mut next_occurrence = Some(last_occurrence);
                let mut iterations = rrule.common_options().interval.unwrap_or(1);
                while iterations > 0 && next_occurrence.is_some() {
                    next_occurrence =
                        self.get_next_occurrence_according_to_rule(next_occurrence.unwrap(), rrule);
                    iterations -= 1;
                }

                next_occurrence
            })
        } else {
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
                    next_non_empty.date().cmp(&exdate.date_time.date()) == Ordering::Equal
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
