use super::*;

#[test]
fn test_issue1_empty_input_does_not_panic() {
    let empty: &[String] = &[];
    let res = Block::try_from(empty);
    assert!(matches!(
        res,
        Err(BlockParseError::BlockNotStartingWithBEGIN)
    ));
}

#[test]
fn test_issue2_unclosed_and_mismatched_blocks_fail() {
    let unclosed = vec!["BEGIN:VCALENDAR".to_string(), "BEGIN:VEVENT".to_string()];
    assert!(Block::try_from(unclosed.as_slice()).is_err());

    let mismatched = vec!["BEGIN:VCALENDAR".to_string(), "END:VEVENT".to_string()];
    assert!(Block::try_from(mismatched.as_slice()).is_err());

    let extra_end = vec![
        "BEGIN:VCALENDAR".to_string(),
        "END:VCALENDAR".to_string(),
        "END:VCALENDAR".to_string(),
    ];
    assert!(Block::try_from(extra_end.as_slice()).is_err());
}

#[test]
fn test_improvement3_single_pass_nested_blocks_from_str() {
    let input = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VTIMEZONE\r\n\
TZID:Europe/Rome\r\n\
BEGIN:STANDARD\r\n\
TZNAME:CET\r\n\
END:STANDARD\r\n\
END:VTIMEZONE\r\n\
END:VCALENDAR\r\n";

    let block = Block::try_from(input).unwrap();
    assert_eq!(block.name, "VCALENDAR");
    assert_eq!(block.inner_lines, vec!["VERSION:2.0"]);
    assert_eq!(block.inner_blocks.len(), 1);
    assert_eq!(block.inner_blocks[0].name, "VTIMEZONE");
    assert_eq!(block.inner_blocks[0].inner_blocks.len(), 1);
    assert_eq!(block.inner_blocks[0].inner_blocks[0].name, "STANDARD");
}
