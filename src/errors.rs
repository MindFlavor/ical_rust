use std::num::ParseIntError;
use thiserror::Error;

// 1. BlockParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BlockParseError {
    #[error("Block must start with BEGIN:")]
    BlockNotStartingWithBEGIN,
    #[error("Unclosed block: {name:?}")]
    UnclosedBlock { name: String },
    #[error("Mismatched END tag: expected {expected:?}, found {found:?}")]
    MismatchedEndTag { expected: String, found: String },
    #[error("Unexpected content after END tag")]
    UnexpectedContentAfterEnd,
}

// 2. ByDayParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ByDayParseError {
    #[error("Invalid weekday {w:?})")]
    InvalidWeekday { w: String },
    #[error("Invalid delta")]
    InvalidDelta(#[from] ParseIntError),
    #[error("Delta cannot be zero")]
    ZeroDelta,
}

// 3. SubstitutionError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SubstitutionError {
    #[error("Cannot construct a date time variant by substituting a Whole day")]
    ConstructingDateTimeBySubstitutingWholeDay,
    #[error("Invalid date or time resulting from substitution")]
    InvalidDateOrTime,
}

// 4. DateIntersectError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DateIntersectError {
    #[error("Start date cannot be after end date")]
    StartDateAfterEndDate,
}

// 5. FrequencyParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FrequencyParseError {
    #[error("Unrecognized frequency {freq:?})")]
    UnrecognizedFrequency { freq: String },
}

// 6. RRuleParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RRuleParseError {
    #[error("Frequency parse error {err:?})")]
    FrequencyParseError { err: FrequencyParseError },
    #[error("Missing frequency token {line:?})")]
    MissingFrequencyToken { line: String },
    #[error("Missing next token after BYMONTH {line:?})")]
    MissingNextTokenAfterByMonth { line: String },
    #[error("ParseIntError")]
    ParseIntError(#[from] ParseIntError),
    #[error("ParseDateOrDateTimeError")]
    ParseDateOrDateTimeError(#[from] chrono::ParseError),
    #[error("Missing either BYDAY or BYMONTHDAY {line:?})")]
    MissingByDayOrByMonthDayError { line: String },
    #[error("Missing BYDAY {line:?})")]
    MissingByDayError { line: String },
    #[error("Invalid INTERVAL value: {interval} (must be >= 1)")]
    InvalidInterval { interval: u32 },
    #[error("Invalid BYMONTH value: {month} (must be 1..=12)")]
    InvalidByMonth { month: u8 },
    #[error("Invalid BYMONTHDAY value: {day} (must be -31..=-1 or 1..=31)")]
    InvalidByMonthDay { day: i8 },
    #[error("Invalid BYSETPOS value: {pos} (must be -366..=-1 or 1..=366)")]
    InvalidBySetPos { pos: i16 },
    #[error("RRULE cannot specify both COUNT and UNTIL")]
    BothCountAndUntilSpecified,
    #[error("ByDayParserError ({error:?}) line == {line:?}")]
    ByDayParserError {
        error: ByDayParseError,
        line: String,
    },
}

// 7. TzIdDateTimeParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TzIdDateTimeParseError {
    #[error("Parse date time error")]
    ChronoParseError(#[from] chrono::ParseError),
    #[error("Ambiguous timezone")]
    AmbiguousTimeZone,
    #[error("Missing TZID= token")]
    MissingTZIDToken,
    #[error("Invalid timezone: {0}")]
    InvalidTimeZone(String),
    #[error("Missing timezone part")]
    MissingTimeZonePart,
    #[error("Missing datetime part")]
    MissingDateTimePart,
}

// 8. VCalendarParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VCalendarParseError {
    #[error("VTimezone parse error")]
    VTimezoneParseError(#[from] VTimezoneParseError),
    #[error("Unsupported tag {tag:?}")]
    UnsupportedTagError { tag: String },
    #[error("VEvent parse error")]
    VEventParseError(#[from] VEventParseError),
    #[error("Block parse error")]
    BlockParseError(#[from] BlockParseError),
}

// 9. VEventParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VEventParseError {
    #[error("Unsupported block tag {tag:?}, expected \"VEVENT\"")]
    UnsupportedTag { tag: String },
    #[error("Missing mandatory colon in line {line:?}")]
    MissingColon { line: String },
    #[error("Missing mandatory semicolon in line {line:?}")]
    MissingSemicolon { line: String },
    #[error("Missing mandatory field {field:?}")]
    MissingMandatoryField { field: String },
    #[error("VEVENT cannot specify both DTEND and DURATION")]
    BothDtEndAndDurationSpecified,
    #[error("Invalid DURATION value: {duration:?}")]
    InvalidDuration { duration: String },
    #[error("Error parsing SEQUENCE number in line {line:?}. Error: {error}")]
    SequenceParseIntError { line: String, error: ParseIntError },
    #[error("Block parse error")]
    BlockParseError(#[from] BlockParseError),
    #[error("RRule parse error")]
    RRuleParseError(#[from] RRuleParseError),
    #[error("TzIdDateTime parse error")]
    TzIdDateTimeParseError(#[from] TzIdDateTimeParseError),
    #[error("Chrono parse error")]
    ChronoParseError(#[from] chrono::ParseError),
}

impl VEventParseError {
    pub fn missing_colon(line: impl Into<String>) -> Self {
        VEventParseError::MissingColon { line: line.into() }
    }
    pub fn missing_semicolon(line: impl Into<String>) -> Self {
        VEventParseError::MissingSemicolon { line: line.into() }
    }
    pub fn missing_mandatory_field(field: impl Into<String>) -> Self {
        VEventParseError::MissingMandatoryField {
            field: field.into(),
        }
    }
    pub fn sequence_parse_int_error(line: impl Into<String>, error: ParseIntError) -> Self {
        VEventParseError::SequenceParseIntError {
            line: line.into(),
            error,
        }
    }
}

// 10. VTimezoneParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VTimezoneParseError {
    #[error("Unsupported block tag {tag:?}, expected \"VTIMEZONE\"")]
    UnsupportedTag { tag: String },
    #[error("TZID tag not found")]
    TZIDTagNotFound,
    #[error("Block parse error")]
    BlockParseError(#[from] BlockParseError),
    #[error("VTimezoneOffset parse error")]
    VTimezoneOffsetParseError(#[from] VTimezoneOffsetParseError),
}

// 11. VTimezoneOffsetParseError
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VTimezoneOffsetParseError {
    #[error("Missing mandatory colon in line {line:?}")]
    MissingColon { line: String },
    #[error("Missing mandatory field {field:?}")]
    MissingMandatoryField { field: &'static str },
    #[error("Unsupported tag {tag:?}")]
    UnsupportedTag { tag: String },
    #[error("Invalid UTC offset: {offset:?}")]
    InvalidOffset { offset: String },
    #[error("Chrono parse error")]
    ChronoParseError(#[from] chrono::ParseError),
    #[error("RRule parse error")]
    RRuleParseError(#[from] RRuleParseError),
}
