use crate::{
    block::Block,
    date_or_date_time::{DateIntersectError, DateOrDateTime, EventOverlap},
    rrule::{RRule, RRuleParseError},
    vevent_iterator::VEventIterator,
    TzIdDateTime,
};
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use std::{num::ParseIntError, ops::Range};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VEventFormatError {
    #[error("Missing mandatory colon (block {block:?})")]
    MissingColon { block: Block },
    #[error("Missing mandatory semicolon (block {block:?})")]
    MissingSemicolon { block: Block },
    #[error("Missing mandatory field {field:?}. Block:\n{block:?}")]
    MissingMandatoryField { block: Block, field: String },
    #[error("Error parsing SEQUENCE number {block:?}. Error: {error}")]
    SequenceParseIntError { block: Block, error: ParseIntError },
    #[error("RRule parse error")]
    RRuleParseError(#[from] RRuleParseError),
    #[error("TzIdDateTime parse error")]
    TzIdDateTimeFormatError(#[from] crate::TzIdDateTimeFormatError),
    #[error("Chrono parse error")]
    ChronoParseError(#[from] chrono::ParseError),
}

impl VEventFormatError {
    pub fn missing_colon(block: Block) -> Self {
        VEventFormatError::MissingColon { block }
    }
    pub fn missing_semicolon(block: Block) -> Self {
        VEventFormatError::MissingSemicolon { block }
    }
    pub fn missing_mandatory_field(block: Block, field: impl Into<String>) -> Self {
        VEventFormatError::MissingMandatoryField {
            field: field.into(),
            block,
        }
    }
    pub fn sequence_parse_int_error(block: Block, error: ParseIntError) -> Self {
        VEventFormatError::SequenceParseIntError { block, error }
    }
}

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
    pub exdates: Vec<TzIdDateTime>,
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
        log::trace!("called next_occurrence_since({:?}, {:?})", self, dt);

        for occurrence in self.into_iter() {
            let event_overlap = {
                // handle the special case of start and end dates being WholeDay. We consider the
                // final date the last second of the previous end date.
                if let (DateOrDateTime::WholeDay(wd_start), DateOrDateTime::WholeDay(wd_end)) =
                    (occurrence.start, occurrence.end)
                {
                    dt.intersects(
                        DateOrDateTime::DateTime(
                            Utc.with_ymd_and_hms(
                                wd_start.year(),
                                wd_start.month(),
                                wd_start.day(),
                                0,
                                0,
                                0,
                            )
                            .unwrap(),
                        ),
                        DateOrDateTime::DateTime(
                            Utc.with_ymd_and_hms(
                                wd_end.year(),
                                wd_end.month(),
                                wd_end.day(),
                                0,
                                0,
                                0,
                            )
                            .unwrap(),
                        ),
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
            // else carry on!
        }

        Ok(None)
    }
}

impl TryFrom<Block> for VEvent {
    type Error = VEventFormatError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        println!("VEvent::try_from({block:?})");

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
            let tag = &line[0..idx_colon];
            let extra = if idx_colon + 1 < line.len() {
                Some(&line[idx_colon + 1..])
            } else {
                None
            };

            println!("tag before == {tag}");

            match tag {
                "LAST-MODIFIED" => {
                    dt_last_modified =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventFormatError::missing_colon(block.clone())
                        })?)?);
                }
                "DTSTART" => {
                    dt_start = Some(DateOrDateTime::DateTime(string_to_datetime(
                        extra.ok_or_else(|| VEventFormatError::missing_colon(block.clone()))?,
                    )?));
                }
                "DTEND" => {
                    dt_end =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventFormatError::missing_colon(block.clone())
                        })?)?);
                }
                "CREATED" => {
                    dt_created =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventFormatError::missing_colon(block.clone())
                        })?)?);
                }
                "DTSTAMP" => {
                    dt_stamp =
                        Some(string_to_date_or_datetime(extra.ok_or_else(|| {
                            VEventFormatError::missing_colon(block.clone())
                        })?)?);
                }
                "SUMMARY" => {
                    summary = Some(
                        extra
                            .ok_or_else(|| VEventFormatError::missing_colon(block.clone()))?
                            .to_string(),
                    );
                }
                "DESCRIPTION" => description = extra.map(|e| e.to_string()),
                "SEQUENCE" => {
                    sequence = extra.map(|e| e.parse::<u32>()).transpose().map_err(|e| {
                        VEventFormatError::sequence_parse_int_error(block.clone(), e)
                    })?;
                }
                "RRULE" => {
                    rrule = Some(
                        extra
                            .ok_or_else(|| VEventFormatError::missing_colon(block.clone()))?
                            .parse::<RRule>()?,
                    );
                }
                "STATUS" => {
                    status = Some(
                        extra
                            .ok_or_else(|| VEventFormatError::missing_colon(block.clone()))?
                            .to_string(),
                    );
                }
                "X-GOOGLE-CONFERENCE" => {
                    google_conference_url = extra.map(|e| e.to_string());
                }
                _ => {} // ignore
            }

            let idx_semicolon = line.find(';').unwrap_or(line.len());
            let tag = &line[0..idx_semicolon];
            let extra = if idx_semicolon + 1 < line.len() {
                Some(&line[idx_semicolon + 1..])
            } else {
                None
            };

            println!("tag second == {tag}, extra == {extra:?}");

            match tag {
                "ORGANIZER" => {
                    organizer = Some(
                        extra
                            .ok_or_else(|| VEventFormatError::missing_colon(block.clone()))?
                            .to_string(),
                    );
                }
                "EXDATE" => {
                    let extra =
                        extra.ok_or_else(|| VEventFormatError::missing_semicolon(block.clone()))?;
                    log::trace!("parsing EXDATE ==> {}", extra);
                    exdates.push(TzIdDateTime::try_from(extra)?);
                }
                "DTSTART" => {
                    dt_start = Some(
                        extra
                            .map(to_tziddate_or_date)
                            .transpose()?
                            .ok_or_else(|| VEventFormatError::missing_semicolon(block.clone()))?,
                    );
                }
                "DTEND" => {
                    dt_end = Some(
                        extra
                            .map(to_tziddate_or_date)
                            .transpose()?
                            .ok_or_else(|| VEventFormatError::missing_semicolon(block.clone()))?,
                    );
                }
                _ => {} // ignore
            }
        }

        let dt_start = dt_start
            .ok_or_else(|| VEventFormatError::missing_mandatory_field(block.clone(), "DTSTART"))?;

        Ok(VEvent {
            dt_last_modified: dt_last_modified.ok_or_else(|| {
                VEventFormatError::missing_mandatory_field(block.clone(), "LAST-MODIFIED")
            })?,
            dt_start,
            dt_end: dt_end.unwrap_or(dt_start), // if there is no DT_END tag, it means end is the same as start.
            dt_created: dt_created.ok_or_else(|| {
                VEventFormatError::missing_mandatory_field(block.clone(), "CREATED")
            })?,
            dt_stamp: dt_stamp.ok_or_else(|| {
                VEventFormatError::missing_mandatory_field(block.clone(), "DTSTAMP")
            })?,
            summary: summary.ok_or_else(|| {
                VEventFormatError::missing_mandatory_field(block.clone(), "SUMMARY")
            })?,
            description,
            rrule,
            exdates,
            sequence: sequence.ok_or_else(|| {
                VEventFormatError::missing_mandatory_field(block.clone(), "SEQUENCE")
            })?,
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
    println!("string_to_date_or_datetime({s})");
    Ok(if s.len() == 8 {
        let date = string_to_date(s)?;
        DateOrDateTime::WholeDay(
            Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                .unwrap(),
        )
    } else {
        DateOrDateTime::DateTime(string_to_datetime(s)?)
    })
}

fn string_to_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    println!("string_to_datetime({s})");
    Ok(if s.ends_with('Z') {
        DateTime::<FixedOffset>::parse_from_str(s, "%Y%m%dT%H%M%S%#z")?.with_timezone(&Utc)
    } else {
        let a = NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S")?;
        let tz_offset = Local::now().offset().to_owned();
        tz_offset
            .from_local_datetime(&a)
            .unwrap()
            .with_timezone(&Utc)
        //Utc.from_utc_datetime(&a)
    })
}

fn string_to_date(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    println!("string_to_date({s})");
    Ok(DateTime::<Local>::from_utc(
        NaiveDateTime::parse_from_str(&format!("{s}T000000"), "%Y%m%dT%H%M%S")?,
        Local::now().offset().to_owned(),
    )
    .with_timezone(&Utc))
}

fn to_tziddate_or_date(
    s: &str,
) -> Result<DateOrDateTime, crate::tzid_date_time::TzIdDateTimeFormatError> {
    println!("to_tziddate_or_date({s})");
    Ok(s.parse::<TzIdDateTime>()?.date_time)
}
