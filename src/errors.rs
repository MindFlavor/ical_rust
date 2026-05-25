use crate::block::Block;
use std::num::ParseIntError;
use thiserror::Error;

// 1. BlockParseError (formerly in block.rs)
#[derive(Error, Debug)]
pub enum BlockParseError {
    #[error("Block must start with BEGIN:")]
    BlockNotStartingWithBEGIN,
}

// 2. ByDayParseError (formerly in by_day.rs)
#[derive(Error, Debug)]
pub enum ByDayParseError {
    #[error("Invalid weekday {w:?})")]
    InvalidWeekday { w: String },
    #[error("Invalid delta")]
    InvalidDelta(#[from] ParseIntError),
}

// 3. SubstitutionError (formerly in date_or_date_time.rs)
#[derive(Error, Debug)]
pub enum SubstitutionError {
    #[error("Cannot construct a date time variant by substituting a Whole day")]
    ConstructingDateTimeBySubstitutingWholeDay,
}

// 4. DateIntersectError (formerly in date_or_date_time.rs)
#[derive(Error, Debug)]
pub enum DateIntersectError {
    #[error("Start date cannot be after end date")]
    StartDateAfterEndDate,
}

// 5. FrequencyParseError (formerly in frequency.rs)
#[derive(Error, Debug)]
pub enum FrequencyParseError {
    #[error("Unrecognized frequency {freq:?})")]
    UnrecognizedFrequency { freq: String },
}

// 6. RRuleParseError (formerly in rrule.rs)
#[derive(Error, Debug)]
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
    #[error("ByDayParserError ({error:?}) line == {line:?}")]
    ByDayParserError {
        error: ByDayParseError,
        line: String,
    },
}

// 7. TzIdDateTimeParseError (formerly TzIdDateTimeFormatError in tzid_date_time.rs)
#[derive(Error, Debug)]
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

// 8. VCalendarParseError (formerly in vcalendar.rs)
#[derive(Error, Debug)]
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

// 9. VEventParseError (formerly VEventFormatError in vevent.rs)
#[derive(Error, Debug)]
pub enum VEventParseError {
    #[error("Missing mandatory colon (block {block:?})")]
    MissingColon { block: Block },
    #[error("Missing mandatory semicolon (block {block:?})")]
    MissingSemicolon { block: Block },
    #[error("Missing mandatory field {field:?}. Block:\n{block:?}")]
    MissingMandatoryField { block: Block, field: String },
    #[error("Error parsing SEQUENCE number {block:?}. Error: {error}")]
    SequenceParseIntError { block: Block, error: ParseIntError },
    #[error("RRule parse error")]
    RRuleParseError(#[from] RRuleParseError),
    #[error("TzIdDateTime parse error")]
    TzIdDateTimeParseError(#[from] TzIdDateTimeParseError),
    #[error("Chrono parse error")]
    ChronoParseError(#[from] chrono::ParseError),
}

impl VEventParseError {
    pub fn missing_colon(block: Block) -> Self {
        VEventParseError::MissingColon { block }
    }
    pub fn missing_semicolon(block: Block) -> Self {
        VEventParseError::MissingSemicolon { block }
    }
    pub fn missing_mandatory_field(block: Block, field: impl Into<String>) -> Self {
        VEventParseError::MissingMandatoryField {
            field: field.into(),
            block,
        }
    }
    pub fn sequence_parse_int_error(block: Block, error: ParseIntError) -> Self {
        VEventParseError::SequenceParseIntError { block, error }
    }
}

// 10. VTimezoneParseError (formerly in vtimezone.rs)
#[derive(Error, Debug)]
pub enum VTimezoneParseError {
    #[error("TZID tag not found")]
    TZIDTagNotFound,
    #[error("VTimezoneOffset parse error")]
    VTimezoneOffsetParseError(#[from] VTimezoneOffsetParseError),
}

// 11. VTimezoneOffsetParseError (formerly in vtimezone.rs)
#[derive(Error, Debug)]
pub enum VTimezoneOffsetParseError {
    #[error("Missing mandatory semicolon (block {block:?})")]
    MissingSemicolon { block: Block },
    #[error("Missing mandatory field {field:?}. Block: {block:?}")]
    MissingMandatoryField { block: Block, field: &'static str },
    #[error("Unsupported tag {tag:?}, Block: {block:?}")]
    UnsupportedTag { block: Block, tag: String },
}
