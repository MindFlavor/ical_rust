use crate::{
    block::Block,
    date_or_date_time::{DateOrDateTime, EventOverlap},
    errors::{DateIntersectError, VEventParseError},
    property::{ContentLine, parse_ical_duration, unescape_ical_text},
    rrule::RRule,
    vevent_iterator::VEventIterator,
};
use chrono::Duration;
use std::ops::Range;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VEvent {
    pub uid: Option<String>,
    pub recurrence_id: Option<DateOrDateTime>,
    pub dt_created: DateOrDateTime,
    pub dt_last_modified: DateOrDateTime,
    pub dt_start: DateOrDateTime,
    pub dt_end: DateOrDateTime,
    pub dt_stamp: DateOrDateTime,
    pub summary: String,
    pub description: Option<String>,
    pub rrule: Option<RRule>,
    pub rdates: Vec<DateOrDateTime>,
    pub exdates: Vec<DateOrDateTime>,
    pub overridden_dates: Vec<DateOrDateTime>,
    pub sequence: u32,
    pub status: Option<String>,
    pub organizer: Option<String>,
    pub google_conference_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceResult {
    pub occurrence: Range<DateOrDateTime>,
    pub event_overlap: EventOverlap,
}

impl VEvent {
    #[must_use]
    pub fn first_occurrence(&self) -> DateOrDateTime {
        self.dt_start
    }

    #[must_use]
    pub fn iter(&self) -> VEventIterator<'_> {
        VEventIterator::new(self)
    }

    pub fn next_occurrence_since(
        &self,
        dt: DateOrDateTime,
    ) -> Result<Option<OccurrenceResult>, DateIntersectError> {
        for occurrence in self.iter() {
            let event_overlap = {
                // handle the special case of start and end dates being WholeDay. We consider the
                // final date the previous day when DTEND > DTSTART (since DTEND is exclusive for DATE).
                if let (DateOrDateTime::WholeDay(wd_start), DateOrDateTime::WholeDay(wd_end)) =
                    (occurrence.start, occurrence.end)
                {
                    let inclusive_end = if wd_end > wd_start {
                        wd_end - Duration::days(1)
                    } else {
                        wd_end
                    };
                    dt.intersects(
                        DateOrDateTime::WholeDay(wd_start),
                        DateOrDateTime::WholeDay(inclusive_end),
                    )?
                } else {
                    dt.intersects(occurrence.start, occurrence.end)?
                }
            };

            log::debug!("event_overlap == {:?} ==> {:?}", occurrence, event_overlap);

            match event_overlap {
                EventOverlap::FinishesPast => {} // carry on
                _ => {
                    return Ok(Some(OccurrenceResult {
                        occurrence,
                        event_overlap,
                    }));
                }
            }
        }

        Ok(None)
    }

    pub fn occurrences_between(
        &self,
        start: DateOrDateTime,
        end: DateOrDateTime,
    ) -> Result<Vec<Range<DateOrDateTime>>, DateIntersectError> {
        if start > end {
            return Err(DateIntersectError::StartDateAfterEndDate);
        }

        let mut results = Vec::new();
        for occurrence in self.iter() {
            let effective_end =
                if let (DateOrDateTime::WholeDay(wd_start), DateOrDateTime::WholeDay(wd_end)) =
                    (occurrence.start, occurrence.end)
                {
                    if wd_end > wd_start {
                        DateOrDateTime::WholeDay(wd_end - Duration::days(1))
                    } else {
                        occurrence.end
                    }
                } else {
                    occurrence.end
                };

            if occurrence.start > end {
                break;
            }
            if effective_end >= start {
                results.push(occurrence);
            }
        }

        Ok(results)
    }
}

impl TryFrom<&str> for VEvent {
    type Error = VEventParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let block = Block::try_from(s)?;
        Self::try_from(block)
    }
}

impl FromStr for VEvent {
    type Err = VEventParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<Block<'_>> for VEvent {
    type Error = VEventParseError;

    fn try_from(block: Block<'_>) -> Result<Self, Self::Error> {
        Self::try_from_block_with_timezones(block, &[])
    }
}

impl VEvent {
    pub fn try_from_block_with_timezones(
        block: Block<'_>,
        timezones: &[crate::vtimezone::VTimezone],
    ) -> Result<Self, VEventParseError> {
        if block.name != "VEVENT" {
            return Err(VEventParseError::UnsupportedTag { tag: block.name });
        }

        let mut uid = None;
        let mut recurrence_id = None;
        let mut dt_created = None;
        let mut dt_last_modified = None;
        let mut dt_start: Option<DateOrDateTime> = None;
        let mut dt_end = None;
        let mut duration = None;
        let mut dt_stamp = None;
        let mut summary = None;
        let mut description = None;
        let mut rrule = None;
        let mut rdates = Vec::new();
        let mut exdates = Vec::new();
        let mut sequence = None;
        let mut status = None;
        let mut organizer = None;
        let mut google_conference_url = None;

        for line in &block.inner_lines {
            let line = line.as_ref();
            let prop =
                ContentLine::parse(line).ok_or_else(|| VEventParseError::missing_colon(line))?;

            if prop.name.eq_ignore_ascii_case("UID") {
                uid = Some(unescape_ical_text(prop.value));
            } else if prop.name.eq_ignore_ascii_case("RECURRENCE-ID") {
                recurrence_id = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("LAST-MODIFIED") {
                dt_last_modified = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("DTSTART") {
                dt_start = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("DTEND") {
                dt_end = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("DURATION") {
                duration = Some(parse_ical_duration(prop.value).ok_or_else(|| {
                    VEventParseError::InvalidDuration {
                        duration: prop.value.to_owned(),
                    }
                })?);
            } else if prop.name.eq_ignore_ascii_case("CREATED") {
                dt_created = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("DTSTAMP") {
                dt_stamp = Some(DateOrDateTime::from_params_and_value_with_timezones(
                    &prop.params,
                    prop.value,
                    timezones,
                )?);
            } else if prop.name.eq_ignore_ascii_case("SUMMARY") {
                summary = Some(unescape_ical_text(prop.value));
            } else if prop.name.eq_ignore_ascii_case("DESCRIPTION") {
                description = Some(unescape_ical_text(prop.value));
            } else if prop.name.eq_ignore_ascii_case("SEQUENCE") {
                sequence = Some(
                    prop.value
                        .parse::<u32>()
                        .map_err(|e| VEventParseError::sequence_parse_int_error(line, e))?,
                );
            } else if prop.name.eq_ignore_ascii_case("RRULE") {
                rrule = Some(prop.value.parse::<RRule>()?);
            } else if prop.name.eq_ignore_ascii_case("STATUS") {
                status = Some(unescape_ical_text(prop.value));
            } else if prop.name.eq_ignore_ascii_case("X-GOOGLE-CONFERENCE") {
                google_conference_url = Some(prop.value.to_string());
            } else if prop.name.eq_ignore_ascii_case("ORGANIZER") {
                organizer = Some(prop.raw_after_semicolon.unwrap_or(prop.value).to_string());
            } else if prop.name.eq_ignore_ascii_case("RDATE") {
                for val in prop.value.split(',').filter(|s| !s.is_empty()) {
                    rdates.push(DateOrDateTime::from_params_and_value_with_timezones(
                        &prop.params,
                        val,
                        timezones,
                    )?);
                }
            } else if prop.name.eq_ignore_ascii_case("EXDATE") {
                for val in prop.value.split(',').filter(|s| !s.is_empty()) {
                    exdates.push(DateOrDateTime::from_params_and_value_with_timezones(
                        &prop.params,
                        val,
                        timezones,
                    )?);
                }
            }
        }

        rdates.sort();
        rdates.dedup();

        let dt_start =
            dt_start.ok_or_else(|| VEventParseError::missing_mandatory_field("DTSTART"))?;
        let dt_stamp = dt_stamp.unwrap_or(dt_start);
        let dt_end = match (dt_end, duration) {
            (Some(_), Some(_)) => return Err(VEventParseError::BothDtEndAndDurationSpecified),
            (Some(end), None) => end,
            (None, Some(dur)) => dt_start + dur,
            (None, None) => match dt_start {
                DateOrDateTime::WholeDay(d) => DateOrDateTime::WholeDay(d + Duration::days(1)),
                DateOrDateTime::DateTime { .. } => dt_start,
            },
        };

        Ok(VEvent {
            uid,
            recurrence_id,
            dt_last_modified: dt_last_modified.unwrap_or(dt_stamp),
            dt_start,
            dt_end,
            dt_created: dt_created.unwrap_or(dt_stamp),
            dt_stamp,
            summary: summary.unwrap_or_default(),
            description,
            rrule,
            rdates,
            exdates,
            overridden_dates: Vec::new(),
            sequence: sequence.unwrap_or(0),
            status,
            organizer,
            google_conference_url,
        })
    }
}

impl<'a> IntoIterator for &'a VEvent {
    type Item = Range<DateOrDateTime>;
    type IntoIter = VEventIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        VEventIterator::new(self)
    }
}

#[cfg(test)]
#[path = "tests/vevent.rs"]
mod tests;
