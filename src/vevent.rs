use crate::{
    block::Block,
    date_or_date_time::{DateOrDateTime, EventOverlap},
    errors::{DateIntersectError, VEventParseError},
    rrule::RRule,
    vevent_iterator::VEventIterator,
};
use chrono::Duration;
use std::ops::Range;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
}

impl TryFrom<Block> for VEvent {
    type Error = VEventParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
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
            let idx_colon = line.find(':').unwrap_or(line.len());
            let name_and_params = &line[0..idx_colon];
            let extra = if idx_colon + 1 < line.len() {
                Some(&line[idx_colon + 1..])
            } else {
                None
            };

            let (tag, after_semicolon) = match name_and_params.find(';') {
                Some(idx_semi) => {
                    let after = if idx_semi + 1 < line.len() {
                        Some(&line[idx_semi + 1..])
                    } else {
                        None
                    };
                    (&name_and_params[0..idx_semi], after)
                }
                None => (name_and_params, None),
            };

            match tag {
                "LAST-MODIFIED" => {
                    dt_last_modified =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventParseError::missing_colon(block.clone())
                        })?)?);
                }
                "DTSTART" => {
                    if let Some(semi_extra) = after_semicolon {
                        dt_start = Some(semi_extra.parse::<DateOrDateTime>()?);
                    } else {
                        dt_start =
                            Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                                VEventParseError::missing_colon(block.clone())
                            })?)?);
                    }
                }
                "DTEND" => {
                    if let Some(semi_extra) = after_semicolon {
                        dt_end = Some(semi_extra.parse::<DateOrDateTime>()?);
                    } else {
                        dt_end =
                            Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                                VEventParseError::missing_colon(block.clone())
                            })?)?);
                    }
                }
                "CREATED" => {
                    dt_created =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventParseError::missing_colon(block.clone())
                        })?)?);
                }
                "DTSTAMP" => {
                    dt_stamp =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventParseError::missing_colon(block.clone())
                        })?)?);
                }
                "SUMMARY" => {
                    summary = Some(
                        extra
                            .ok_or_else(|| VEventParseError::missing_colon(block.clone()))?
                            .to_string(),
                    );
                }
                "DESCRIPTION" => description = extra.map(|e| e.to_string()),
                "SEQUENCE" => {
                    sequence = extra.map(|e| e.parse::<u32>()).transpose().map_err(|e| {
                        VEventParseError::sequence_parse_int_error(block.clone(), e)
                    })?;
                }
                "RRULE" => {
                    rrule = Some(
                        extra
                            .ok_or_else(|| VEventParseError::missing_colon(block.clone()))?
                            .parse::<RRule>()?,
                    );
                }
                "STATUS" => {
                    status = Some(
                        extra
                            .ok_or_else(|| VEventParseError::missing_colon(block.clone()))?
                            .to_string(),
                    );
                }
                "X-GOOGLE-CONFERENCE" => {
                    google_conference_url = extra.map(|e| e.to_string());
                }
                "ORGANIZER" => {
                    if let Some(semi_extra) = after_semicolon {
                        organizer = Some(semi_extra.to_string());
                    } else {
                        organizer = Some(
                            extra
                                .ok_or_else(|| VEventParseError::missing_colon(block.clone()))?
                                .to_string(),
                        );
                    }
                }
                "EXDATE" => {
                    if let Some(semi_extra) = after_semicolon {
                        log::trace!("parsing EXDATE ==> {}", semi_extra);

                        if let Some(colon_idx) = semi_extra.find(':') {
                            let prefix = &semi_extra[0..=colon_idx];
                            let values_str = &semi_extra[colon_idx + 1..];
                            for val in values_str.split(',').filter(|s| !s.is_empty()) {
                                let reconstructed = format!("{}{}", prefix, val);
                                exdates.push(DateOrDateTime::try_from(reconstructed.as_str())?);
                            }
                        } else {
                            exdates.push(DateOrDateTime::try_from(semi_extra)?);
                        }
                    } else {
                        let vals =
                            extra.ok_or_else(|| VEventParseError::missing_colon(block.clone()))?;
                        for val in vals.split(',').filter(|s| !s.is_empty()) {
                            exdates.push(string_to_date_or_datetime(val)?);
                        }
                    }
                }
                _ => {} // ignore
            }
        }

        let dt_start = dt_start
            .ok_or_else(|| VEventParseError::missing_mandatory_field(block.clone(), "DTSTART"))?;
        let dt_stamp = dt_stamp.unwrap_or(dt_start);

        Ok(VEvent {
            dt_last_modified: dt_last_modified.unwrap_or(dt_stamp),
            dt_start,
            dt_end: dt_end.unwrap_or(dt_start),
            dt_created: dt_created.unwrap_or(dt_stamp),
            dt_stamp,
            summary: summary.ok_or_else(|| {
                VEventParseError::missing_mandatory_field(block.clone(), "SUMMARY")
            })?,
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
