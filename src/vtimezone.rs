use crate::errors::{VTimezoneOffsetParseError, VTimezoneParseError};
use crate::property::ContentLine;
use crate::{block::Block, rrule::RRule};
use chrono::{FixedOffset, NaiveDate};

#[derive(Debug, Clone)]
pub struct VTimezone {
    pub tz_id: String,
    pub offsets: Vec<VTimezoneOffset>,
}

#[derive(Debug, Clone)]
pub struct VTimezoneOffset {
    pub tz_name: Option<String>,
    pub tz_offset_from: FixedOffset,
    pub tz_offset_to: FixedOffset,
    pub dt_start: NaiveDate,
    pub rrule: Option<RRule>,
}

pub fn parse_utc_offset(s: &str) -> Result<FixedOffset, VTimezoneOffsetParseError> {
    let bytes = s.as_bytes();
    if bytes.len() != 5 && bytes.len() != 7 {
        return Err(VTimezoneOffsetParseError::InvalidOffset {
            offset: s.to_owned(),
        });
    }
    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => {
            return Err(VTimezoneOffsetParseError::InvalidOffset {
                offset: s.to_owned(),
            });
        }
    };
    let hours: i32 = s.get(1..3).and_then(|v| v.parse().ok()).ok_or_else(|| {
        VTimezoneOffsetParseError::InvalidOffset {
            offset: s.to_owned(),
        }
    })?;
    let minutes: i32 = s.get(3..5).and_then(|v| v.parse().ok()).ok_or_else(|| {
        VTimezoneOffsetParseError::InvalidOffset {
            offset: s.to_owned(),
        }
    })?;
    let seconds: i32 = if bytes.len() == 7 {
        s.get(5..7).and_then(|v| v.parse().ok()).ok_or_else(|| {
            VTimezoneOffsetParseError::InvalidOffset {
                offset: s.to_owned(),
            }
        })?
    } else {
        0
    };

    if hours > 23 || minutes > 59 || seconds > 59 {
        return Err(VTimezoneOffsetParseError::InvalidOffset {
            offset: s.to_owned(),
        });
    }

    let total_secs = sign * (hours * 3600 + minutes * 60 + seconds);
    FixedOffset::east_opt(total_secs).ok_or_else(|| VTimezoneOffsetParseError::InvalidOffset {
        offset: s.to_owned(),
    })
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
            .filter(|b| b.name == "STANDARD" || b.name == "DAYLIGHT")
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
                "TZOFFSETFROM" => tz_offset_from = Some(parse_utc_offset(prop.value)?),
                "TZOFFSETTO" => tz_offset_to = Some(parse_utc_offset(prop.value)?),
                "DTSTART" => {
                    dt_start = Some(
                        chrono::NaiveDateTime::parse_from_str(prop.value, "%Y%m%dT%H%M%S")?.date(),
                    )
                }
                "RRULE" => rrule = Some(prop.value.parse()?),
                _ => {} // ignore optional RFC 5545 properties such as COMMENT, RDATE, X-*
            }
        }

        Ok(Self {
            tz_name,
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
}
