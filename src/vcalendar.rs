use crate::VEvent;
use crate::block::Block;
use crate::errors::VCalendarParseError;
use crate::ical_line_parser::ICalLineParser;
use crate::vtimezone::VTimezone;
use either::*;

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
        if block.name != "VCALENDAR" {
            return Err(VCalendarParseError::UnsupportedTagError { tag: block.name });
        }

        let results = block
            .inner_blocks
            .into_iter()
            .filter_map(|b| match b.name.as_ref() {
                "VTIMEZONE" => Some(
                    VTimezone::try_from(b)
                        .map_err(VCalendarParseError::from)
                        .map(Left),
                ),
                "VEVENT" => Some(
                    VEvent::try_from(b)
                        .map_err(VCalendarParseError::from)
                        .map(Right),
                ),
                _ => None,
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
        assert_eq!(
            occurrences[1].start.year(),
            2022,
            "Second occurrence should be in 2022 (December)"
        );
        assert_eq!(
            occurrences[1].start.month(),
            12,
            "Second occurrence should be December (12)"
        );

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
        assert_eq!(
            occurrences[1].start.year(),
            2023,
            "Second occurrence should be in 2023 (January)"
        );
        assert_eq!(
            occurrences[1].start.month(),
            1,
            "Second occurrence should be January (1)"
        );

        // Occurrence 3: Mar 5, 2023 (+2 months)
        assert_eq!(occurrences[2].start.year(), 2023);
        assert_eq!(
            occurrences[2].start.month(),
            3,
            "Third occurrence should be March (3)"
        );
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
        assert!(
            result.is_err(),
            "Expected an error instead of panic or successful parse"
        );
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
        assert!(
            result.is_err(),
            "Expected an error instead of panic or successful parse"
        );
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
        assert!(
            result.is_err(),
            "Expected an error instead of panic or successful parse"
        );
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

    #[test]
    fn test_issue7_dst_transition_preserves_local_wall_clock_time() {
        // Europe/Rome transitions from CET (UTC+1) to CEST (UTC+2) on 2023-03-26
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20230325T090000\r\n\
LAST-MODIFIED:20230325T080000Z\r\n\
CREATED:20230325T080000Z\r\n\
DTSTAMP:20230325T080000Z\r\n\
SUMMARY:Morning Standup Across DST\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;COUNT=2\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let event = &cal.events[0];
        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 2);

        // 2023-03-25 09:00 CET (UTC+1) == 08:00 UTC
        assert_eq!(occurrences[0].start.hour(), 8);
        // 2023-03-26 09:00 CEST (UTC+2) == 07:00 UTC
        assert_eq!(occurrences[1].start.hour(), 7);
    }

    #[test]
    fn test_issue8_monthly_by_day_same_month_and_day1() {
        // Jan 2023: Jan 30 is Monday. Feb 2023: Feb 1 is Wednesday.
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20230127\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Monthly Mon/Wed/Fri\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;BYDAY=MO,WE,FR;COUNT=3\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let occurrences: Vec<_> = cal.events[0].into_iter().collect();
        assert_eq!(occurrences.len(), 3);

        // 1st: Fri Jan 27, 2023
        assert_eq!(
            (occurrences[0].start.month(), occurrences[0].start.day()),
            (1, 27)
        );
        // 2nd: Mon Jan 30, 2023 (same month!)
        assert_eq!(
            (occurrences[1].start.month(), occurrences[1].start.day()),
            (1, 30)
        );
        // 3rd: Wed Feb 1, 2023 (day 1 of next month!)
        assert_eq!(
            (occurrences[2].start.month(), occurrences[2].start.day()),
            (2, 1)
        );
    }

    #[test]
    fn test_issue9_yearly_by_month_month_day_and_leap_year() {
        // Leap day event: Feb 29, 2024 with YearlyByMonthByMonthDay
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20240229\r\n\
LAST-MODIFIED:20240101T000000Z\r\n\
CREATED:20240101T000000Z\r\n\
DTSTAMP:20240101T000000Z\r\n\
SUMMARY:Leap Day Event\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29;COUNT=5\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let occurrences: Vec<_> = cal.events[0].into_iter().collect();
        assert_eq!(occurrences.len(), 5);
        // 2024 is leap year -> Feb 29
        assert_eq!(
            (
                occurrences[0].start.year(),
                occurrences[0].start.month(),
                occurrences[0].start.day()
            ),
            (2024, 2, 29)
        );
        // 2028 is leap year -> should return to Feb 29, not drift to Feb 28!
        assert_eq!(
            (
                occurrences[4].start.year(),
                occurrences[4].start.month(),
                occurrences[4].start.day()
            ),
            (2028, 2, 29)
        );
    }

    #[test]
    fn test_issue10_exdate_date_comparison_ignores_time() {
        // Event with DateTime at 09:00 UTC, and EXDATE;VALUE=DATE:20230102 (midnight 00:00 UTC)
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230101T090000Z\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Daily Meeting with Date Exdate\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;COUNT=2\r\n\
EXDATE;VALUE=DATE:20230102\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let occurrences: Vec<_> = cal.events[0].into_iter().collect();
        assert_eq!(occurrences.len(), 2);
        // Jan 2 should be excluded, so the 2nd occurrence must be Jan 3
        assert_eq!(occurrences[0].start.day(), 1);
        assert_eq!(occurrences[1].start.day(), 3);
    }

    #[test]
    fn test_issue11_vevent_property_parameters_and_plain_tags() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230101\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY;LANGUAGE=en-US:Team Sync; Planning\r\n\
DESCRIPTION;LANGUAGE=en-US:Notes for the meeting\r\n\
ORGANIZER:mailto:boss@example.com\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;COUNT=2\r\n\
EXDATE:20230102\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let event = &cal.events[0];
        assert_eq!(event.summary, "Team Sync; Planning");
        assert_eq!(event.description.as_deref(), Some("Notes for the meeting"));
        assert_eq!(event.organizer.as_deref(), Some("mailto:boss@example.com"));
        let occurrences: Vec<_> = event.into_iter().collect();
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start.day(), 1);
        assert_eq!(occurrences[1].start.day(), 3);
    }

    #[test]
    fn test_issue13_rrule_flexible_token_order_and_monthly_defaults() {
        // 1. FREQ not first
        let rrule1: crate::rrule::RRule = "COUNT=3;FREQ=DAILY".parse().unwrap();
        assert!(matches!(rrule1, crate::rrule::RRule::Daily(_)));

        // 2. FREQ=YEARLY;BYMONTH=6 without BYDAY/BYMONTHDAY
        let rrule2: crate::rrule::RRule = "FREQ=YEARLY;BYMONTH=6;COUNT=2".parse().unwrap();
        assert!(matches!(
            rrule2,
            crate::rrule::RRule::YearlyByMonthByMonthDay(_)
        ));

        // 3. FREQ=MONTHLY without BYDAY/BYMONTHDAY and negative BYMONTHDAY=-1
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20230115\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Plain Monthly\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;COUNT=3\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
DTSTART;VALUE=DATE:20230131\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Last Day of Month\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=MONTHLY;BYMONTHDAY=-1;COUNT=3\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let occ0: Vec<_> = cal.events[0].into_iter().collect();
        assert_eq!(occ0.len(), 3);
        assert_eq!((occ0[1].start.month(), occ0[1].start.day()), (2, 15));
        assert_eq!((occ0[2].start.month(), occ0[2].start.day()), (3, 15));

        let occ1: Vec<_> = cal.events[1].into_iter().collect();
        assert_eq!(occ1.len(), 3);
        assert_eq!((occ1[0].start.month(), occ1[0].start.day()), (1, 31));
        assert_eq!((occ1[1].start.month(), occ1[1].start.day()), (2, 28));
        assert_eq!((occ1[2].start.month(), occ1[2].start.day()), (3, 31));
    }

    #[test]
    fn test_issue14_first_occurrence_respects_until_and_count() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230110T090000Z\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Zero Count Event\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;COUNT=0\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230110T090000Z\r\n\
LAST-MODIFIED:20230101T000000Z\r\n\
CREATED:20230101T000000Z\r\n\
DTSTAMP:20230101T000000Z\r\n\
SUMMARY:Expired Until Event\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=DAILY;UNTIL=20230101T000000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        let occ0: Vec<_> = cal.events[0].into_iter().collect();
        assert!(occ0.is_empty());

        let occ1: Vec<_> = cal.events[1].into_iter().collect();
        assert!(occ1.is_empty());
    }

    #[test]
    fn test_issue16_vevent_optional_created_last_modified_sequence() {
        let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230101T090000Z\r\n\
DTSTAMP:20230101T080000Z\r\n\
SUMMARY:Minimal Event\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let cal = VCalendar::try_from(ics_content).unwrap();
        assert_eq!(cal.events.len(), 1);
        let event = &cal.events[0];
        assert_eq!(event.summary, "Minimal Event");
        assert_eq!(event.sequence, 0);
        assert_eq!(event.dt_created, event.dt_stamp);
        assert_eq!(event.dt_last_modified, event.dt_stamp);
    }

    #[test]
    fn test_issue17_vcalendar_validates_root_tag_and_ignores_other_components() {
        // Non-VCALENDAR root block must be rejected
        let not_a_calendar =
            "BEGIN:VEVENT\r\nDTSTART:20230101T090000Z\r\nSUMMARY:Test\r\nEND:VEVENT";
        assert!(VCalendar::try_from(not_a_calendar).is_err());

        // VCALENDAR containing VTODO alongside VEVENT should succeed and ignore VTODO
        let with_vtodo = "BEGIN:VCALENDAR\r\n\
BEGIN:VTODO\r\n\
SUMMARY:Buy milk\r\n\
END:VTODO\r\n\
BEGIN:VEVENT\r\n\
DTSTART:20230101T090000Z\r\n\
SUMMARY:Actual Event\r\n\
END:VEVENT\r\n\
END:VCALENDAR";
        let cal = VCalendar::try_from(with_vtodo).unwrap();
        assert_eq!(cal.events.len(), 1);
        assert_eq!(cal.events[0].summary, "Actual Event");
    }
}
