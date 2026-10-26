use super::*;

#[test]
fn parse_content_line_with_multiple_and_quoted_params() {
    let line =
        ContentLine::parse("DTSTART;VALUE=DATE-TIME;TZID=\"Europe/Rome\":20230101T090000").unwrap();
    assert_eq!(line.name, "DTSTART");
    assert_eq!(line.param("VALUE"), Some("DATE-TIME"));
    assert_eq!(line.param("TZID"), Some("Europe/Rome"));
    assert_eq!(line.value, "20230101T090000");
}

#[test]
fn unescape_rfc5545_text() {
    let raw = r"Line 1\nLine 2\NComma\, Semicolon\; Backslash\\";
    assert_eq!(
        unescape_ical_text(raw),
        "Line 1\nLine 2\nComma, Semicolon; Backslash\\"
    );
}
