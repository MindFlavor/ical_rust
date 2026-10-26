use super::*;

#[test]
fn unfolded_lines_are_borrowed_and_folded_lines_are_owned() {
    let input = "BEGIN:VCALENDAR\r\nSUMMARY:Hello \r\n World\r\n\t!\r\nEND:VCALENDAR";
    let parsed: Vec<Cow<'_, str>> = ICalLineParser::new(input).collect();
    assert_eq!(parsed.len(), 3);
    assert!(matches!(parsed[0], Cow::Borrowed("BEGIN:VCALENDAR")));
    assert!(matches!(parsed[1], Cow::Owned(_)));
    assert_eq!(parsed[1], "SUMMARY:Hello World!");
    assert!(matches!(parsed[2], Cow::Borrowed("END:VCALENDAR")));
}
