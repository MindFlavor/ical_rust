use crate::{block::Block, rrule::RRule};
use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct VTimezone {
    pub tz_id: String,
    pub offsets: Vec<VTimezoneOffset>, // TODO: populate!
}

#[derive(Error, Debug)]
pub enum VTimezoneParseError {
    #[error("TZID tag not found")]
    TZIDTagNotFound,
    #[error("VTimezoneOffset parse error")]
    VTimezoneOffsetParseError(#[from] VTimezoneOffsetParseError),
}

#[derive(Error, Debug)]
pub enum VTimezoneOffsetParseError {
    #[error("Missing mandatory semicolon (block {block:?})")]
    MissingSemicolon { block: Block },
    #[error("Missing mandatory field {field:?}. Block: {block:?}")]
    MissingMandatoryField { block: Block, field: &'static str },
    #[error("Unsupported tag {tag:?}, Block: {block:?}")]
    UnsupportedTag { block: Block, tag: String },
}

#[derive(Debug, Clone)]
pub struct VTimezoneOffset {
    pub tz_name: String,
    pub tz_offset_from: String,
    pub tz_offset_to: String,
    pub dt_start: NaiveDate,
    pub rrule: Option<RRule>,
}

impl TryFrom<Block> for VTimezone {
    type Error = VTimezoneParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let tz_id = block
            .inner_lines
            .iter()
            .find_map(|l| l.strip_prefix("TZID:"))
            .ok_or(VTimezoneParseError::TZIDTagNotFound)?
            .to_owned();

        let offsets = block
            .inner_blocks
            .into_iter()
            .map(VTimezoneOffset::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(VTimezone { tz_id, offsets })
    }
}

impl TryFrom<Block> for VTimezoneOffset {
    type Error = VTimezoneOffsetParseError;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let mut tz_name = None;
        let mut tz_offset_from = None;
        let mut tz_offset_to = None;
        let mut dt_start = None;
        let mut rrule = None;

        for s in &block.inner_lines {
            let mut tokens = s.split(':');
            let key = tokens
                .next()
                .ok_or_else(|| VTimezoneOffsetParseError::MissingSemicolon {
                    block: block.to_owned(),
                })?;
            let value = tokens.collect::<Vec<_>>().join(":");

            match key {
                "TZNAME" => tz_name = Some(value),
                "TZOFFSETFROM" => tz_offset_from = Some(value),
                "TZOFFSETTO" => tz_offset_to = Some(value),
                "DTSTART" => {
                    dt_start = Some(NaiveDate::parse_from_str(&value, "%Y%m%dT%H%M%S").unwrap())
                }
                "RRULE" => rrule = Some(value.parse().unwrap()),

                _ => {
                    return Err(VTimezoneOffsetParseError::UnsupportedTag {
                        block: block.clone(),
                        tag: key.to_owned(),
                    })
                }
            }
        }

        Ok(Self {
            tz_name: tz_name.ok_or_else(|| VTimezoneOffsetParseError::MissingMandatoryField {
                block: block.to_owned(),
                field: "TZNAME",
            })?,
            tz_offset_from: tz_offset_from.ok_or_else(|| {
                VTimezoneOffsetParseError::MissingMandatoryField {
                    block: block.to_owned(),
                    field: "TZOFFSETFROM",
                }
            })?,
            tz_offset_to: tz_offset_to.ok_or_else(|| {
                VTimezoneOffsetParseError::MissingMandatoryField {
                    block: block.to_owned(),
                    field: "TZOFFSETTO",
                }
            })?,
            dt_start: dt_start.ok_or_else(|| VTimezoneOffsetParseError::MissingMandatoryField {
                block: block.to_owned(),
                field: "DTSTART",
            })?,
            rrule,
        })
    }
}
