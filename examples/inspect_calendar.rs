use chrono::{DateTime, Local, NaiveDate, Utc};
use ical_rust::{DateOrDateTime, EventOverlap, VCalendar};

fn main() {
    let e = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsPastEndsSameDay
    );

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/mindflavor/tmp/basic.ics.1".to_owned());
    let Ok(whole_file) = std::fs::read_to_string(path) else {
        return;
    };

    let cal = VCalendar::try_from(whole_file.as_str()).unwrap();
    let dt = DateOrDateTime::date_time(Utc::now());
    println!("\n\tdt == {dt:?}");

    for event in &cal.events {
        if let Some(next_occurrence) = event.next_occurrence_since(dt).unwrap() {
            match next_occurrence.event_overlap {
                EventOverlap::StartsFuture | EventOverlap::FinishesPast => {}
                _ => {
                    let a = next_occurrence.occurrence.start.as_datetime();
                    let local = a.with_timezone(&Local);

                    println!(
                        "event.summary \"{}\" ==> {next_occurrence:?} (local : {local:?})",
                        event.summary
                    );
                }
            }
        }
    }
}
