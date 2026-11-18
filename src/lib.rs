pub mod adapters;
pub mod config;
pub mod covers;
pub mod ini;
pub mod launcher;
pub mod psp;
pub mod state;
pub mod scanner;
pub mod wave;

slint::include_modules!();

/// Local wall clock without pulling in a timezone crate.
pub fn clock_strings() -> (slint::SharedString, slint::SharedString) {
    use chrono::{Datelike, Local, Timelike};
    let n = Local::now();
    (
        format!("{:02}:{:02}", n.hour(), n.minute()).into(),
        format!("{}, {} {}", n.format("%A"), n.day(), n.format("%B")).into(),
    )
}

pub fn month() -> u32 {
    use chrono::{Datelike, Local};
    Local::now().month()
}
