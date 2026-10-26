use super::*;

#[test]
fn test_issue3_malformed_dtstart_or_rrule_in_vtimezone_returns_err() {
    let bad_dtstart_block = Block {
        name: "STANDARD".to_string(),
        inner_lines: vec![
            "TZNAME:CET".to_string(),
            "TZOFFSETFROM:+0200".to_string(),
            "TZOFFSETTO:+0100".to_string(),
            "DTSTART:INVALID_DATE".to_string(),
        ],
        inner_blocks: vec![],
    };
    assert!(VTimezoneOffset::try_from(bad_dtstart_block).is_err());

    let bad_rrule_block = Block {
        name: "STANDARD".to_string(),
        inner_lines: vec![
            "TZNAME:CET".to_string(),
            "TZOFFSETFROM:+0200".to_string(),
            "TZOFFSETTO:+0100".to_string(),
            "DTSTART:19701025T030000".to_string(),
            "RRULE:INVALID_RRULE".to_string(),
        ],
        inner_blocks: vec![],
    };
    assert!(VTimezoneOffset::try_from(bad_rrule_block).is_err());
}

#[test]
fn test_improvement5_typed_offsets_and_optional_tzname() {
    let block = Block::try_from(
        "BEGIN:VTIMEZONE\r\n\
TZID:Europe/Rome\r\n\
BEGIN:STANDARD\r\n\
TZOFFSETFROM:+0200\r\n\
TZOFFSETTO:+0100\r\n\
DTSTART:19701025T030000\r\n\
COMMENT:Optional comment without TZNAME\r\n\
END:STANDARD\r\n\
END:VTIMEZONE",
    )
    .unwrap();

    let vtz = VTimezone::try_from(block).unwrap();
    assert_eq!(vtz.tz_id, "Europe/Rome");
    assert_eq!(vtz.offsets.len(), 1);
    assert_eq!(vtz.offsets[0].tz_name, None);
    assert_eq!(
        vtz.offsets[0].tz_offset_from,
        FixedOffset::east_opt(2 * 3600).unwrap()
    );
    assert_eq!(
        vtz.offsets[0].tz_offset_to,
        FixedOffset::east_opt(3600).unwrap()
    );
}
