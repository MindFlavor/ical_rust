#![feature(iter_advance_by)]

mod block;
mod by_day;
mod date_or_date_time;
mod frequency;
mod ical_line_parser;
mod rrule;
pub mod tzid_date_time;
mod vcalendar;
mod vevent;
mod vevent_iterator;
mod vtimezone;

use crate::ical_line_parser::ICalLineParser;
use block::Block;
use chrono::{Date, Local, Utc};
pub use date_or_date_time::*;
use std::collections::HashMap;
pub use tzid_date_time::*;
pub use vcalendar::*;
pub use vevent::*;

fn main() {
    let whole_file = std::fs::read_to_string("/home/mindflavor/tmp/basic.ics").unwrap();
    let contents = whole_file.split("\r\n").collect::<Vec<_>>();
    let ical_lines: &[String] = &ICalLineParser::new(&contents).collect::<Vec<_>>();
    //println!("ical_lines == {:?}", ical_lines);

    let block: Block = ical_lines.try_into().unwrap();
    println!("block == {:?}\n", block);

    let hm = block.inner_blocks.iter().map(|b| b.name()).fold(
        HashMap::new(),
        |mut accum: HashMap<&str, u32>, item| {
            let v = accum.entry(item).or_insert(0);
            *v += 1;
            accum
        },
    );
    println!("hm== {:?}\n", hm);

    block
        .inner_blocks
        .iter()
        .filter(|b| b.name == "VTIMEZONE")
        .for_each(|b| println!("b == {:?}", b));

    let cal: VCalendar = whole_file.as_str().try_into().unwrap();
    println!("\n cal== {:?}\n", cal);

    //let v_calendar = VCalendar::try_from(contents).unwrap();

    ////println!("v_calendar == {:?}\n", v_calendar);

    //let uscita_lisa = v_calendar
    //    .events
    //    .iter()
    //    .filter(|item| item.summary == "Uscita Lisa")
    //    .collect::<Vec<_>>();

    //for uscita in uscita_lisa {
    //    println!("{:?}\n", uscita);
    //}

    ////let no_sequence_cnt = v_calendar
    ////    .events
    ////    .iter()
    ////    .filter(|item| item.sequence == 4)
    ////    .count();
    ////println!("no_sequence_cnt = {}", no_sequence_cnt);

    ////let mut rrules = v_calendar
    ////    .events
    ////    .iter()
    ////    .filter(|item| item.rrule.is_some())
    ////    .map(|item| item.rrule.as_ref().unwrap())
    ////    .fold(HashMap::new(), |mut hm: HashMap<&RRule, u32>, item| {
    ////        let val = hm.entry(item).or_default();
    ////        *val += 1;
    ////        hm
    ////    })
    ////    .into_iter()
    ////    .collect::<Vec<_>>();

    ////rrules.sort_by(|(_, val1), (_, val2)| val2.cmp(val1));

    ////println!("rrules = {:?}", rrules);

    ////println!("unhandled:");
    ////for item in rrules.iter().filter(|(rrule, _)| match rrule {
    ////    RRule::Generic(_) => true,
    ////    _ => false,
    ////}) {
    ////    println!("item == {:?}", item);
    ////}

    ////for item in v_calendar.events.iter().filter(|i| i.dt_end.is_none()) {
    ////    println!("{:?}", item);
    ////}

    //println!();

    //let list = v_calendar
    //    .events
    //    .iter()
    //    //.filter(|i| matches!(i.rrule, Some(RRule::Yearly(_))))
    //    .filter(|e| e.summary == "Ritiro bimbe dal bus")
    //    .collect::<Vec<_>>();

    //println!("found {} items!", list.len());

    //for (i, item) in list.iter().enumerate() {
    //    println!("item [{}] == {:?}", i, item);
    //}

    //let item = list[0];

    //println!("\n{:?}", item);

    //return;

    //for occurrence in item.into_iter() {
    //    println!("occurrence == {:?}", occurrence);
    //}

    //let dt = DateTime::parse_from_str("20220119T103000Z", "%Y%m%dT%H%M%S%#z")
    //    .unwrap()
    //    .with_timezone(&Utc);

    //item.next_occurrence_since(dt).unwrap();

    // find occurrences tomorrow!
    for delta in 2..4 {
        let dt = Utc::now().date() + chrono::Duration::days(delta);

        println!("\n\tdt == {:?}", dt);

        for event in cal.events.iter() {
            let next_occurrence = event.next_occurrence_since(dt).unwrap();
            if let Some(next_occurrence) = next_occurrence {
                match next_occurrence.event_overlap {
                    EventOverlap::StartsFuture | EventOverlap::FinishesPast => continue,
                    _ => {
                        let a = match next_occurrence.occurrence.start {
                            DateOrDateTime::DateTime(dt) => dt,
                            DateOrDateTime::WholeDay(wd) => wd.and_hms(0, 0, 0),
                        };
                        let local = a.with_timezone(&Local);

                        println!(
                            "event {} ==> {:?} (local : {:?})",
                            event.summary, next_occurrence.occurrence, local
                        );
                    }
                }
            }
        }
    }

    let events_to_check = cal
        .events
        .iter()
        .filter(|e| e.summary == "Esame pap??")
        .collect::<Vec<_>>();

    println!("\nevents_to_check == {:#?}", events_to_check);

    //let dt = Utc::now().date() + chrono::Duration::days(3);
    //println!("\nevent to check == {:?}", event_to_check);
    //let next_occurrence = event_to_check.next_occurrence_since(dt).unwrap();
    //println!("next_occurrence == {:?}", next_occurrence);

    ////let mut curr = Some(item.first_occurrence());
    ////while let Some(start) = curr {
    ////    println!("{:?}", start);
    ////    curr = item.next_occurrence(start);
    ////}
}
