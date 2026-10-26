use crate::errors::{VTimezoneOffsetParseError, VTimezoneParseError};
use crate::property::ContentLine;
use crate::{block::Block, rrule::RRule};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct VTimezone {
    pub tz_id: String,
    pub offsets: Vec<VTimezoneOffset>,
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
            .filter_map(|l| ContentLine::parse(l))
            .find(|p| p.name == "TZID")
            .map(|p| p.value.to_owned())
            .ok_or(VTimezoneParseError::TZIDTagNotFound)?;

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
            let prop = ContentLine::parse(s).ok_or_else(|| {
                VTimezoneOffsetParseError::MissingSemicolon {
                    block: block.to_owned(),
                }
            })?;

            match prop.name {
                "TZNAME" => tz_name = Some(prop.value.to_owned()),
                "TZOFFSETFROM" => tz_offset_from = Some(prop.value.to_owned()),
                "TZOFFSETTO" => tz_offset_to = Some(prop.value.to_owned()),
                "DTSTART" => {
                    dt_start = Some(
                        chrono::NaiveDateTime::parse_from_str(prop.value, "%Y%m%dT%H%M%S")?.date(),
                    )
                }
                "RRULE" => rrule = Some(prop.value.parse()?),

                _ => {
                    return Err(VTimezoneOffsetParseError::UnsupportedTag {
                        block: block.clone(),
                        tag: prop.name.to_owned(),
                    });
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

#[cfg(test)]
mod tests {
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
}
