use crate::VEvent;
use crate::block::Block;
use crate::date_or_date_time::DateOrDateTime;
use crate::errors::{DateIntersectError, VCalendarParseError};
use crate::property::{ContentLine, unescape_ical_text};
use crate::vevent::OccurrenceResult;
use crate::vtimezone::VTimezone;
use std::ops::Range;
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VCalendar {
    pub version: Option<String>,
    pub prod_id: Option<String>,
    pub calscale: Option<String>,
    pub method: Option<String>,
    pub timezones: Vec<VTimezone>,
    pub events: Vec<VEvent>,
}

impl VCalendar {
    pub fn occurrences_between(
        &self,
        start: DateOrDateTime,
        end: DateOrDateTime,
    ) -> Result<Vec<(&VEvent, Range<DateOrDateTime>)>, DateIntersectError> {
        if start > end {
            return Err(DateIntersectError::StartDateAfterEndDate);
        }

        let mut results = Vec::new();
        for event in &self.events {
            for occ in event.occurrences_between(start, end)? {
                results.push((event, occ));
            }
        }
        results.sort_by_key(|a| a.1.start);
        Ok(results)
    }

    pub fn next_occurrences_since(
        &self,
        dt: DateOrDateTime,
    ) -> Result<Vec<(&VEvent, OccurrenceResult)>, DateIntersectError> {
        let mut results = Vec::new();
        for event in &self.events {
            if let Some(occ) = event.next_occurrence_since(dt)? {
                results.push((event, occ));
            }
        }
        results.sort_by_key(|a| a.1.occurrence.start);
        Ok(results)
    }
}

impl TryFrom<&str> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(whole_text: &str) -> Result<Self, Self::Error> {
        let block = Block::try_from(whole_text)?;
        block.try_into()
    }
}

impl FromStr for VCalendar {
    type Err = VCalendarParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<Block> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        if block.name != "VCALENDAR" {
            return Err(VCalendarParseError::UnsupportedTagError { tag: block.name });
        }

        let mut version = None;
        let mut prod_id = None;
        let mut calscale = None;
        let mut method = None;

        for line in &block.inner_lines {
            if let Some(prop) = ContentLine::parse(line) {
                match prop.name.to_ascii_uppercase().as_str() {
                    "VERSION" => version = Some(prop.value.to_owned()),
                    "PRODID" => prod_id = Some(unescape_ical_text(prop.value)),
                    "CALSCALE" => calscale = Some(prop.value.to_owned()),
                    "METHOD" => method = Some(prop.value.to_owned()),
                    _ => {}
                }
            }
        }

        let mut timezones = Vec::new();
        let mut event_blocks = Vec::new();

        for b in block.inner_blocks {
            match b.name.as_str() {
                "VTIMEZONE" => timezones.push(VTimezone::try_from(b)?),
                "VEVENT" => event_blocks.push(b),
                _ => {}
            }
        }

        let mut events = Vec::with_capacity(event_blocks.len());
        for b in event_blocks {
            events.push(VEvent::try_from_block_with_timezones(b, &timezones)?);
        }

        // Link RECURRENCE-ID overrides to master recurring VEVENTs sharing the same UID
        let overrides: Vec<(String, DateOrDateTime)> = events
            .iter()
            .filter_map(|ev| Some((ev.uid.clone()?, ev.recurrence_id?)))
            .collect();

        if !overrides.is_empty() {
            for ev in &mut events {
                if ev.recurrence_id.is_none()
                    && let Some(uid) = &ev.uid
                {
                    for (ov_uid, rec_id) in &overrides {
                        if uid == ov_uid && !ev.overridden_dates.contains(rec_id) {
                            ev.overridden_dates.push(*rec_id);
                        }
                    }
                }
            }
        }

        Ok(Self {
            version,
            prod_id,
            calscale,
            method,
            timezones,
            events,
        })
    }
}

#[cfg(test)]
#[path = "tests/vcalendar.rs"]
mod tests;
