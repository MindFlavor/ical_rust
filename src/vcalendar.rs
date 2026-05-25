use crate::block::Block;
use crate::ical_line_parser::ICalLineParser;
use crate::vtimezone::VTimezone;
use crate::VEvent;
use either::*;
use crate::errors::VCalendarParseError;

#[derive(Debug, Clone, Default)]
pub struct VCalendar {
    pub timezones: Vec<VTimezone>,
    pub events: Vec<VEvent>,
}


impl TryFrom<&str> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(whole_text: &str) -> Result<Self, Self::Error> {
        let normalized = whole_text.replace("\r\n", "\n");
        let contents = normalized.split('\n').collect::<Vec<_>>();
        let ical_lines: &[String] = &ICalLineParser::new(&contents).collect::<Vec<_>>();
        let block: Block = ical_lines.try_into()?;

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

    #[test]
    fn test_bug4_yearly_by_month_by_day() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20221124T090000\r\n\
LAST-MODIFIED:20221124T170000Z\r\n\
CREATED:20221124T170000Z\r\n\
DTSTAMP:20221124T170000Z\r\n\
SUMMARY:Thanksgiving Dinner\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=4TH;COUNT=3\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];

        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 3);

        // Occurrence 1: Nov 24, 2022
        assert_eq!(occurrences[0].start.year(), 2022);
        assert_eq!(occurrences[0].start.month(), 11);
        assert_eq!(occurrences[0].start.day(), 24);

        // Occurrence 2: Nov 23, 2023
        assert_eq!(occurrences[1].start.year(), 2023);
        assert_eq!(occurrences[1].start.month(), 11);
        assert_eq!(occurrences[1].start.day(), 23);

        // Occurrence 3: Nov 28, 2024
        assert_eq!(occurrences[2].start.year(), 2024);
        assert_eq!(occurrences[2].start.month(), 11);
        assert_eq!(occurrences[2].start.day(), 28);
    }

    #[test]
    fn test_bug5_exdate_multi_value() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20230101T090000\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:Daily Standup\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;COUNT=5\r\n\
EXDATE;TZID=Europe/Rome:20230102T090000,20230104T090000\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];

        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 5, "Expected exactly 5 occurrences total");

        // Occurrence 1: Jan 1, 2023
        assert_eq!(occurrences[0].start.year(), 2023);
        assert_eq!(occurrences[0].start.month(), 1);
        assert_eq!(occurrences[0].start.day(), 1);

        // Occurrence 2: Jan 3, 2023 (Jan 2 is excluded)
        assert_eq!(occurrences[1].start.year(), 2023);
        assert_eq!(occurrences[1].start.month(), 1);
        assert_eq!(occurrences[1].start.day(), 3);

        // Occurrence 3: Jan 5, 2023 (Jan 4 is excluded)
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(occurrences[2].start.month(), 1);
        assert_eq!(occurrences[2].start.day(), 5);

        // Occurrence 4: Jan 6, 2023
        assert_eq!(occurrences[3].start.year(), 2023);
        assert_eq!(occurrences[3].start.month(), 1);
        assert_eq!(occurrences[3].start.day(), 6);

        // Occurrence 5: Jan 7, 2023
        assert_eq!(occurrences[4].start.year(), 2023);
        assert_eq!(occurrences[4].start.month(), 1);
        assert_eq!(occurrences[4].start.day(), 7);
    }

    #[test]
    fn test_bug6_missing_begin_tag_fails_safely() {
        let ics_content = "INVALID CALENDAR CONTENT\r\nSUMMARY:Test";
        let result = VCalendar::try_from(ics_content);
        assert!(result.is_err(), "Expected an error instead of panic or successful parse");
    }

    #[test]
    fn test_bug7_lf_line_endings_supported() {
        let ics_content = "BEGIN:VCALENDAR\n\
BEGIN:VEVENT\n\
DTSTART;TZID=Europe/Rome:20230101T090000\n\
LAST-MODIFIED:20230101T170000Z\n\
CREATED:20230101T170000Z\n\
DTSTAMP:20230101T170000Z\n\
SUMMARY:LF Line Ending Test\n\
SEQUENCE:0\n\
END:VEVENT\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        assert_eq!(cal.events[0].summary, "LF Line Ending Test");
    }

    #[test]
    fn test_bug8_tab_line_folding_supported() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20230101T090000\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:LF Line Ending \r\n\
\twith Tab Folding\r\n\
SEQUENCE:0\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        assert_eq!(cal.events[0].summary, "LF Line Ending with Tab Folding");
    }

    #[test]
    fn test_bug9_month_skipping_for_end_of_month_dates() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20230131\r\n\
LAST-MODIFIED:20230131T170000Z\r\n\
CREATED:20230131T170000Z\r\n\
DTSTAMP:20230131T170000Z\r\n\
SUMMARY:End of Month Recurrence\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;BYMONTHDAY=31;COUNT=5\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];

        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 5);

        // Occurrence 1: Jan 31, 2023
        assert_eq!(occurrences[0].start.year(), 2023);
        assert_eq!(occurrences[0].start.month(), 1);
        assert_eq!(occurrences[0].start.day(), 31);

        // Occurrence 2: Feb 28, 2023 (clamped)
        assert_eq!(occurrences[1].start.year(), 2023);
        assert_eq!(occurrences[1].start.month(), 2);
        assert_eq!(occurrences[1].start.day(), 28);

        // Occurrence 3: Mar 31, 2023
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(occurrences[2].start.month(), 3);
        assert_eq!(occurrences[2].start.day(), 31);

        // Occurrence 4: Apr 30, 2023 (clamped)
        assert_eq!(occurrences[3].start.year(), 2023);
        assert_eq!(occurrences[3].start.month(), 4);
        assert_eq!(occurrences[3].start.day(), 30);

        // Occurrence 5: May 31, 2023
        assert_eq!(occurrences[4].start.year(), 2023);
        assert_eq!(occurrences[4].start.month(), 5);
        assert_eq!(occurrences[4].start.day(), 31);
    }

    #[test]
    fn test_bug10_weekly_by_day_interval_skipping() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20230103\r\n\
LAST-MODIFIED:20230103T170000Z\r\n\
CREATED:20230103T170000Z\r\n\
DTSTAMP:20230103T170000Z\r\n\
SUMMARY:Bi-Weekly Multi-Day Meeting\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH;COUNT=4\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];

        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 4);

        // Occurrence 1: Tue Jan 3, 2023
        assert_eq!(occurrences[0].start.year(), 2023);
        assert_eq!(occurrences[0].start.month(), 1);
        assert_eq!(occurrences[0].start.day(), 3);

        // Occurrence 2: Thu Jan 5, 2023
        assert_eq!(occurrences[1].start.year(), 2023);
        assert_eq!(occurrences[1].start.month(), 1);
        assert_eq!(occurrences[1].start.day(), 5);

        // Occurrence 3: Tue Jan 17, 2023 (skipped week of Jan 9-15)
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(occurrences[2].start.month(), 1);
        assert_eq!(occurrences[2].start.day(), 17);

        // Occurrence 4: Thu Jan 19, 2023
        assert_eq!(occurrences[3].start.year(), 2023);
        assert_eq!(occurrences[3].start.month(), 1);
        assert_eq!(occurrences[3].start.day(), 19);
    }
}




