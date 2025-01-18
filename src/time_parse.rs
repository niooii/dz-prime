use std::collections::HashSet;
use chrono::{NaiveTime, Timelike};
use crate::model::DayOfWeek;

fn parse_dayofweek(c: char) -> Option<DayOfWeek> {
    match c {
        'U' => Some(DayOfWeek::Sunday),
        'M' => Some(DayOfWeek::Monday),
        'T' => Some(DayOfWeek::Tuesday), 
        'W' => Some(DayOfWeek::Wednesday),
        'R' => Some(DayOfWeek::Thursday),
        'F' => Some(DayOfWeek::Friday),
        'S' => Some(DayOfWeek::Saturday),
        _ => None
    }
}

fn parse_repeat_at(token: &String) -> Option<i32> {
    println!("{token}");
    if let Err(e) = NaiveTime::parse_from_str(&token, "%I%p") {
        eprintln!("{e}");
    }
    // Bypass not enough info for unique time
    let alt_token = format!("{token}:00");
    // Chain try parsing
    let time = if let Ok(t) = NaiveTime::parse_from_str(&token, "%I:%M%p") {
        // Try 12h format first (9:30am/pm)
        t
    } else if let Ok(t) = NaiveTime::parse_from_str(&alt_token, "%I%p:%M") {
        // Try 12h format first (9am/pm)
        t
    } else {
        return None;
    };

    Some((time.hour() * 60 + time.minute()) as i32)
}

/// Returns a tuple of (remind_at, on_days, repeat_weekly)
pub fn parse_time_string(time_str: String) -> (i32, HashSet<DayOfWeek>, bool) {
    let tokens: Vec<String> = time_str.split(" ").map(String::from).collect();

    (parse_repeat_at(&tokens[0]).unwrap(), HashSet::new(), true)
}