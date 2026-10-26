use super::*;

#[test]
fn parse_00() {
    let s = "MO,TU,FR";
    let rules = parse_by_day(s).unwrap();
    assert_eq!(
        rules,
        vec![
            WeekdayRule::plain(Weekday::Mon),
            WeekdayRule::plain(Weekday::Tue),
            WeekdayRule::plain(Weekday::Fri),
        ]
    );
}

#[test]
fn parse_delta() {
    let r1: WeekdayRule = "-20MO".parse().unwrap();
    assert_eq!(r1, WeekdayRule::with_offset(-20, Weekday::Mon));
    let r2: WeekdayRule = "30FR".parse().unwrap();
    assert_eq!(r2, WeekdayRule::with_offset(30, Weekday::Fri));
}

#[test]
fn parse_multi_delta() {
    let rules = parse_by_day("1MO,3MO,-1FR").unwrap();
    assert_eq!(
        rules,
        vec![
            WeekdayRule::with_offset(1, Weekday::Mon),
            WeekdayRule::with_offset(3, Weekday::Mon),
            WeekdayRule::with_offset(-1, Weekday::Fri),
        ]
    );
}

#[test]
fn test_issue4_zero_delta_rejected_and_does_not_panic() {
    assert!(parse_by_day("0MO").is_err());
}

#[test]
fn test_issue5_utf8_input_does_not_panic() {
    assert!(parse_by_day("1€").is_err());
    assert!(parse_by_day("1ÖM").is_err());
    assert!(parse_by_day("1MO,INVALID").is_err());
}
