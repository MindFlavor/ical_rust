use crate::block::Block;
use crate::ical_line_parser::ICalLineParser;
use crate::vtimezone::{VTimezone, VTimezoneParseError};
use crate::VEvent;
use either::*;
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct VCalendar {
    pub timezones: Vec<VTimezone>,
    pub events: Vec<VEvent>,
}

#[derive(Error, Debug)]
pub enum VCalendarParseError {
    #[error("VTimezone parse error")]
    VTimezoneParseError(#[from] VTimezoneParseError),
    #[error("Unsupported tag {tag:?}")]
    UnsupportedTagError { tag: String },
    #[error("VEvent parse error")]
    VEventFormatError(#[from] crate::vevent::VEventFormatError),
}

impl TryFrom<&str> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(whole_text: &str) -> Result<Self, Self::Error> {
        let contents = whole_text.split("\r\n").collect::<Vec<_>>();
        let ical_lines: &[String] = &ICalLineParser::new(&contents).collect::<Vec<_>>();
        let block: Block = ical_lines.try_into().unwrap();

        block.try_into()
    }
}

impl TryFrom<Block> for VCalendar {
    type Error = VCalendarParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let results = block
            .inner_blocks
            .into_iter()
            .map(|b| match b.name.as_ref() {
                "VTIMEZONE" => VTimezone::try_from(b)
                    .map_err(VCalendarParseError::from)
                    .map(Left),
                "VEVENT" => VEvent::try_from(b)
                    .map_err(VCalendarParseError::from)
                    .map(Right),
                _ => Err(VCalendarParseError::UnsupportedTagError {
                    tag: b.name().to_owned(),
                }),
            })
            .collect::<Result<Vec<_>, VCalendarParseError>>()?;

        let mut timezones = Vec::new();
        let mut events = Vec::new();

        for result in results {
            match result {
                Either::Left(timezone) => timezones.push(timezone),
                Either::Right(event) => events.push(event),
            }
        }

        Ok(Self { timezones, events })
    }
}
