use crate::DateOrDateTime;
use chrono::{Local, NaiveDateTime, TimeZone, Utc};

#[test]
fn test_issue12_floating_local_datetime_uses_event_offset() {
    unsafe {
        std::env::set_var("TZ", "Europe/Rome");
    }
    let winter_naive = NaiveDateTime::parse_from_str("20230115T120000", "%Y%m%dT%H%M%S").unwrap();
    let summer_naive = NaiveDateTime::parse_from_str("20230715T120000", "%Y%m%dT%H%M%S").unwrap();

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
        DateOrDateTime::parse_value_str("20230115T120000")
            .unwrap()
            .as_datetime(),
        expected_winter
    );
    assert_eq!(
        DateOrDateTime::parse_value_str("20230715T120000")
            .unwrap()
            .as_datetime(),
        expected_summer
    );
}
