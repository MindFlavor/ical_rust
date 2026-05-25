# ical_rust 📅

[![Crates.io](https://img.shields.io/crates/v/ical_rust.svg)](https://crates.io/crates/ical_rust)
[![Documentation](https://docs.rs/ical_rust/badge.svg)](https://docs.rs/ical_rust)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`ical_rust` is a lightweight, efficient, and robust Rust library for parsing iCalendar (`.ics`) files and dynamically expanding recurring events. It simplifies scheduling logic by fully evaluating complex recurrence rules (`RRULE`) and exclusion dates (`EXDATE`) to calculate concrete event occurrences.

---

## 🚀 Features

- **iCalendar Parser**: Robust parsing of `.ics` file streams, automatically handling folded lines and block structures.
- **Recurrence Expansion**: Comprehensive support for recurring events (`RRULE`), supporting `YEARLY`, `MONTHLY`, `WEEKLY`, and `DAILY` frequencies, intervals, count limiters, until dates, specific weekdays (`BYDAY`), and day of month (`BYMONTHDAY`).
- **Exclusion Dates (`EXDATE`)**: Supports exclusion lists to accurately skip specified recurrence dates.
- **Timezone Integration**: High-fidelity timezone management backed by the robust `chrono` and `chrono-tz` libraries.
- **Collision & Overlap Queries**: Evaluate active events or find next occurrences with simple APIs.

---

## 🛠️ Quick Start

Here is a simple example showing how to parse an iCalendar string, access an event, and expand its recurring occurrences:

```rust
use ical_rust::{VCalendar, Options};

fn main() {
    // A sample ICS payload containing a recurring event
    let ics_content = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
DTSTART;TZID=Europe/Rome:20230101T090000\r\n\
LAST-MODIFIED:20230101T170000Z\r\n\
CREATED:20230101T170000Z\r\n\
DTSTAMP:20230101T170000Z\r\n\
SUMMARY:Rust Pair Programming\r\n\
SEQUENCE:0\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=10\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

    // 1. Parse the iCalendar content
    let cal = VCalendar::try_from(ics_content).expect("Failed to parse calendar");

    // 2. Access the parsed event
    let event = &cal.events[0];
    println!("Calendar Event Found: {}", event.summary);

    // 3. Iterate over the calculated occurrences
    println!("\nExpanding Recurrence Rule ({}):", event.rrule.as_ref().unwrap().common_options().raw);
    for (i, occurrence) in event.into_iter().enumerate() {
        println!("  Occurrence #{}:", i + 1);
        println!("    Start: {:?}", occurrence.start);
        println!("    End:   {:?}", occurrence.end);
    }
}
```

> [!NOTE]
> Standard iCalendar payloads use CRLF (`\r\n`) line endings. `ical_rust` relies on CRLF line separators to accurately split and parse properties.

---

## 🔍 Core Architecture

### `VCalendar`
The main entrance point and container for parsed iCalendar information. It manages parsed `VTimezone` collections and lists of `VEvent` records.
- Implementations of `TryFrom<&str>` allow direct instantiation from file-read contents.

### `VEvent`
Represents an individual event block (`VEVENT`). Contains standard properties:
- `dt_start` / `dt_end`: Event lifetime boundaries as `DateOrDateTime`.
- `summary` / `description`: Metadata explaining the event.
- `rrule`: Optional recurrence rules.
- `exdates`: Optional list of excluded date-times.

You can query individual events using:
- **`into_iter()`**: Returns a `VEventIterator` that generates each concrete recurrence instance of the event as a `Range<DateOrDateTime>`.
- **`next_occurrence_since(dt)`**: Finds the next occurrence of the event that is active or starts after a specified time.

### `DateOrDateTime`
A custom enum handling both single full-day bounds and precise timezone-associated date-times:
- `DateOrDateTime::WholeDay(DateTime<Utc>)`
- `DateOrDateTime::DateTime(DateTime<Utc>)`

Provides support for calculations, incrementing components safely (e.g. leap years, variable-length months), and testing for intersection overlapping (`intersects`).

---

## 🧪 Running Tests

The library contains a robust test suite covering edge cases in timezone calculation, rule iteration, and parser syntax. You can run them locally using:

```bash
cargo test
```

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](file:///home/mindflavor/src/rust/ical_rust/LICENSE) file for details.
