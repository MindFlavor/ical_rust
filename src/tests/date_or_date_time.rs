use super::*;

#[test]
fn parse_tzid_00() {
    let s = "TZID=Europe/Rome:20220106T154000";
    let dt: DateOrDateTime = s.try_into().unwrap();
    assert_eq!(dt.timezone(), Some(chrono_tz::Europe::Rome));
}

#[test]
fn parse_tzid_01() {
    let s = "TZID=Europe/Rome:20211006T170000";
    let dt: DateOrDateTime = s.try_into().unwrap();
    assert_eq!(dt.timezone(), Some(chrono_tz::Europe::Rome));
}

#[test]
fn inc_month_simple() {
    let date = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 5).unwrap());
    let date_time = DateOrDateTime::date_time(Utc.with_ymd_and_hms(2022, 2, 5, 10, 30, 0).unwrap());

    let next = date.inc_month(1);
    assert_eq!(date.year(), next.year());
    assert_eq!(date.month() + 1, next.month());

    let next = date_time.inc_month(12);
    assert_eq!(date_time.year() + 1, next.year());
    assert_eq!(date_time.month(), next.month());
}

#[test]
fn next_weekday() {
    let date = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 5).unwrap()); // SAT

    assert_eq!(date + Duration::days(6), date.next_weekday(Weekday::Fri));
    assert_eq!(date + Duration::days(1), date.next_weekday(Weekday::Sun));
    assert_eq!(date + Duration::days(7), date.next_weekday(Weekday::Sat));
}

#[test]
fn next_weekdays() {
    let date = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 5).unwrap()); // SAT

    assert_eq!(
        date + Duration::days(1),
        date.next_weekdays(&[Weekday::Fri, Weekday::Sun])
    );
    assert_eq!(
        date + Duration::days(6),
        date.next_weekdays(&[Weekday::Fri])
    );
    assert_eq!(
        date + Duration::days(6),
        date.next_weekdays(&[Weekday::Fri, Weekday::Sat])
    );
    assert_eq!(
        date + Duration::days(2),
        date.next_weekdays(&[Weekday::Mon, Weekday::Fri])
    );
}

#[test]
fn move_by_day() {
    let date = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 5).unwrap());

    let first_sunday = date.move_by_delta(&WeekdayRule::with_offset(1, Weekday::Sun));
    assert_eq!(first_sunday.day(), 6);

    let last_sunday = date.move_by_delta(&WeekdayRule::with_offset(-1, Weekday::Sun));
    assert_eq!(last_sunday.day(), 27);
}

#[test]
fn check_intersects_date() {
    let e = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::FinishesPast
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsFuture
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsPastEndsFuture
    );

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

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartSameDayEndsSameDay
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsSameDayEndsFuture
    );

    // Date instead of DateTime
    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2025, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsSameDayEndsFuture
    );

    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 1).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsPastEndsSameDay
    );

    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartSameDayEndsSameDay
    );
}

#[test]
fn check_intersects_date_time() {
    let e = DateOrDateTime::date_time(Utc.with_ymd_and_hms(2022, 2, 10, 8, 0, 0).unwrap());

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::FinishesPast
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20300201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsFuture
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220201T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20390205T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsPastEndsFuture
    );

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

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T103000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartSameDayEndsSameDay
    );

    let dt_start = DateOrDateTime::date_time(
        DateTime::parse_from_str("20220210T023000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let dt_end = DateOrDateTime::date_time(
        DateTime::parse_from_str("20250210T183000Z", "%Y%m%dT%H%M%S%#z")
            .unwrap()
            .with_timezone(&Utc),
    );
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsSameDayEndsFuture
    );

    // Date instead of DateTime
    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2025, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsSameDayEndsFuture
    );

    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 1).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartsPastEndsSameDay
    );

    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());
    assert_eq!(
        e.intersects(dt_start, dt_end).unwrap(),
        EventOverlap::StartSameDayEndsSameDay
    );
}

#[test]
fn test_issue15_intersects_start_after_end_always_errors() {
    let query_day = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 1).unwrap());
    let dt_start = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 20).unwrap());
    let dt_end = DateOrDateTime::WholeDay(NaiveDate::from_ymd_opt(2022, 2, 10).unwrap());

    assert!(matches!(
        query_day.intersects(dt_start, dt_end),
        Err(DateIntersectError::StartDateAfterEndDate)
    ));

    let query_dt = DateOrDateTime::date_time(Utc.with_ymd_and_hms(2022, 2, 1, 0, 0, 0).unwrap());
    let dt_start_dt =
        DateOrDateTime::date_time(Utc.with_ymd_and_hms(2022, 2, 20, 0, 0, 0).unwrap());
    let dt_end_dt = DateOrDateTime::date_time(Utc.with_ymd_and_hms(2022, 2, 10, 0, 0, 0).unwrap());

    assert!(matches!(
        query_dt.intersects(dt_start_dt, dt_end_dt),
        Err(DateIntersectError::StartDateAfterEndDate)
    ));
}

#[test]
fn test_ord_eq_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let utc_instant = Utc.with_ymd_and_hms(2023, 1, 1, 8, 0, 0).unwrap();
    let a = DateOrDateTime::date_time(utc_instant);
    let b = DateOrDateTime::date_time_with_tz(utc_instant, chrono_tz::Europe::Rome);

    assert_eq!(a.cmp(&b), Ordering::Equal);
    assert_eq!(a, b);

    let hash_of = |val: DateOrDateTime| {
        let mut h = DefaultHasher::new();
        val.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash_of(a), hash_of(b));
}
