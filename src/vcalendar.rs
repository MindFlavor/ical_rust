use crate::block::Block;
use crate::ical_line_parser::ICalLineParser;
use crate::vtimezone::{VTimezone, VTimezoneParseError};
use crate::VEvent;
use either::*;
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct VCalendar {
    pub timezones: Vec<VTimezone>,
    pub events: Vec<VEvent>,
}

#[derive(Error, Debug)]
pub enum VCalendarParseError {
    #[error("VTimezone parse error")]
    VTimezoneParseError(#[from] VTimezoneParseError),
    #[error("Unsupported tag {tag:?}")]
    UnsupportedTagError { tag: String },
    #[error("VEvent parse error")]
    VEventFormatError(#[from] crate::vevent::VEventFormatError),
}

impl TryFrom<&str> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(whole_text: &str) -> Result<Self, Self::Error> {
        let contents = whole_text.split("\r\n").collect::<Vec<_>>();
        let ical_lines: &[String] = &ICalLineParser::new(&contents).collect::<Vec<_>>();
        let block: Block = ical_lines.try_into().unwrap();

        block.try_into()
    }
}

impl TryFrom<Block> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let results = block
            .inner_blocks
            .into_iter()
            .map(|b| match b.name.as_ref() {
                "VTIMEZONE" => VTimezone::try_from(b)
                    .map_err(VCalendarParseError::from)
                    .map(Left),
                "VEVENT" => VEvent::try_from(b)
                    .map_err(VCalendarParseError::from)
                    .map(Right),
                _ => Err(VCalendarParseError::UnsupportedTagError {
                    tag: b.name().to_owned(),
                }),
            })
            .collect::<Result<Vec<_>, VCalendarParseError>>()?;

        let mut timezones = Vec::new();
        let mut events = Vec::new();

        for result in results {
            match result {
                Either::Left(timezone) => timezones.push(timezone),
                Either::Right(event) => events.push(event),
            }
        }

        Ok(Self { timezones, events })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readme_example() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20230101T090000\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:Rust Pair Programming\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=10\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];
        assert_eq!(event.summary, "Rust Pair Programming");

        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 10);
    }

    #[test]
    fn test_bug1_monthly_recurrence_december() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20221105\r\n\
LAST-MODIFIED:20221105T170000Z\r\n\
CREATED:20221105T170000Z\r\n\
DTSTAMP:20221105T170000Z\r\n\
SUMMARY:Monthly Team Sync\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;BYMONTHDAY=5;COUNT=3\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let event = &cal.events[0];
        
        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 3);
        
        // Occurrence 1: Nov 5, 2022
        assert_eq!(occurrences[0].start.year(), 2022);
        assert_eq!(occurrences[0].start.month(), 11);
        
        // Occurrence 2: Dec 5, 2022
        assert_eq!(occurrences[1].start.year(), 2022, "Second occurrence should be in 2022 (December)");
        assert_eq!(occurrences[1].start.month(), 12, "Second occurrence should be December (12)");
        
        // Occurrence 3: Jan 5, 2023
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(occurrences[2].start.month(), 1);
    }

    #[test]
    fn test_bug2_double_interval_monthly() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20221105\r\n\
LAST-MODIFIED:20221105T170000Z\r\n\
CREATED:20221105T170000Z\r\n\
DTSTAMP:20221105T170000Z\r\n\
SUMMARY:Bi-Monthly Team Sync\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=5;COUNT=3\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let event = &cal.events[0];
        
        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 3);
        
        // Occurrence 1: Nov 5, 2022
        assert_eq!(occurrences[0].start.year(), 2022);
        assert_eq!(occurrences[0].start.month(), 11);
        
        // Occurrence 2: Jan 5, 2023 (+2 months)
        assert_eq!(occurrences[1].start.year(), 2023, "Second occurrence should be in 2023 (January)");
        assert_eq!(occurrences[1].start.month(), 1, "Second occurrence should be January (1)");
        
        // Occurrence 3: Mar 5, 2023 (+2 months)
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(occurrences[2].start.month(), 3, "Third occurrence should be March (3)");
    }

    #[test]
    fn test_bug3_invalid_timezone_fails_safely() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Invalid/Timezone:20230101T090000\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:Invalid TZID test\r\n\
SEQUENCE:0\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let result = VCalendar::try_from(ics_content);
        assert!(result.is_err(), "Expected an error instead of panic or successful parse");
    }

    #[test]
    fn test_bug3_missing_datetime_fails_safely() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:Missing datetime test\r\n\
SEQUENCE:0\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let result = VCalendar::try_from(ics_content);
        assert!(result.is_err(), "Expected an error instead of panic or successful parse");
    }
}




