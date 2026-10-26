use crate::{
    block::Block,
    date_or_date_time::{DateOrDateTime, EventOverlap},
    errors::{DateIntersectError, VEventParseError},
    property::{ContentLine, unescape_ical_text},
    rrule::RRule,
    vevent_iterator::VEventIterator,
};
use chrono::Duration;
use std::ops::Range;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VEvent {
    pub dt_created: DateOrDateTime,
    pub dt_last_modified: DateOrDateTime,
    pub dt_start: DateOrDateTime,
    pub dt_end: DateOrDateTime,
    pub dt_stamp: DateOrDateTime,
    pub summary: String,
    pub description: Option<String>,
    pub rrule: Option<RRule>,
    pub exdates: Vec<DateOrDateTime>,
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
    pub fn first_occurrence(&self) -> DateOrDateTime {
        self.dt_start
    }

    pub fn next_occurrence_since(
        &self,
        dt: DateOrDateTime,
    ) -> Result<Option<OccurrenceResult>, DateIntersectError> {
        for occurrence in self.into_iter() {
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
        for occurrence in self.into_iter() {
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

impl TryFrom<Block> for VEvent {
    type Error = VEventParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        if block.name != "VEVENT" {
            return Err(VEventParseError::UnsupportedTag { tag: block.name });
        }

        let mut dt_created = None;
        let mut dt_last_modified = None;
        let mut dt_start: Option<DateOrDateTime> = None;
        let mut dt_end = None;
        let mut dt_stamp = None;
        let mut summary = None;
        let mut description = None;
        let mut rrule = None;
        let mut exdates = Vec::new();
        let mut sequence = None;
        let mut status = None;
        let mut organizer = None;
        let mut google_conference_url = None;

        for line in block.inner_lines.iter() {
            let prop =
                ContentLine::parse(line).ok_or_else(|| VEventParseError::missing_colon(line))?;

            match prop.name {
                "LAST-MODIFIED" => {
                    dt_last_modified = Some(DateOrDateTime::from_params_and_value(
                        &prop.params,
                        prop.value,
                    )?);
                }
                "DTSTART" => {
                    dt_start = Some(DateOrDateTime::from_params_and_value(
                        &prop.params,
                        prop.value,
                    )?);
                }
                "DTEND" => {
                    dt_end = Some(DateOrDateTime::from_params_and_value(
                        &prop.params,
                        prop.value,
                    )?);
                }
                "CREATED" => {
                    dt_created = Some(DateOrDateTime::from_params_and_value(
                        &prop.params,
                        prop.value,
                    )?);
                }
                "DTSTAMP" => {
                    dt_stamp = Some(DateOrDateTime::from_params_and_value(
                        &prop.params,
                        prop.value,
                    )?);
                }
                "SUMMARY" => {
                    summary = Some(unescape_ical_text(prop.value));
                }
                "DESCRIPTION" => {
                    description = Some(unescape_ical_text(prop.value));
                }
                "SEQUENCE" => {
                    sequence = Some(
                        prop.value
                            .parse::<u32>()
                            .map_err(|e| VEventParseError::sequence_parse_int_error(line, e))?,
                    );
                }
                "RRULE" => {
                    rrule = Some(prop.value.parse::<RRule>()?);
                }
                "STATUS" => {
                    status = Some(unescape_ical_text(prop.value));
                }
                "X-GOOGLE-CONFERENCE" => {
                    google_conference_url = Some(prop.value.to_string());
                }
                "ORGANIZER" => {
                    organizer = Some(prop.raw_after_semicolon.unwrap_or(prop.value).to_string());
                }
                "EXDATE" => {
                    for val in prop.value.split(',').filter(|s| !s.is_empty()) {
                        exdates.push(DateOrDateTime::from_params_and_value(&prop.params, val)?);
                    }
                }
                _ => {} // ignore
            }
        }

        let dt_start =
            dt_start.ok_or_else(|| VEventParseError::missing_mandatory_field("DTSTART"))?;
        let dt_stamp = dt_stamp.unwrap_or(dt_start);

        Ok(VEvent {
            dt_last_modified: dt_last_modified.unwrap_or(dt_stamp),
            dt_start,
            dt_end: dt_end.unwrap_or(dt_start),
            dt_created: dt_created.unwrap_or(dt_stamp),
            dt_stamp,
            summary: summary.ok_or_else(|| VEventParseError::missing_mandatory_field("SUMMARY"))?,
            description,
            rrule,
            exdates,
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

pub(crate) fn string_to_date_or_datetime(s: &str) -> Result<DateOrDateTime, chrono::ParseError> {
    DateOrDateTime::parse_value_str(s)
}

#[cfg(test)]
mod tests {
    use super::string_to_date_or_datetime;
    use chrono::{Local, NaiveDateTime, TimeZone, Utc};

    #[test]
    fn test_issue12_floating_local_datetime_uses_event_offset() {
        unsafe {
            std::env::set_var("TZ", "Europe/Rome");
        }
        let winter_naive =
            NaiveDateTime::parse_from_str("20230115T120000", "%Y%m%dT%H%M%S").unwrap();
        let summer_naive =
            NaiveDateTime::parse_from_str("20230715T120000", "%Y%m%dT%H%M%S").unwrap();

        let expected_winter = Local
            .from_local_datetime(&winter_naive)
            .earliest()
            .unwrap()
            .with_timezone(&Utc);
        let expected_summer = Local
            .from_local_datetime(&summer_naive)
            .earliest()
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(
            string_to_date_or_datetime("20230115T120000")
                .unwrap()
                .as_datetime(),
            expected_winter
        );
        assert_eq!(
            string_to_date_or_datetime("20230715T120000")
                .unwrap()
                .as_datetime(),
            expected_summer
        );
    }
}
