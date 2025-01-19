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

fn parse_on_days(token: &String) -> Option<HashSet<DayOfWeek>> {
    if token.is_empty() {
        return None;
    }
    let upper = token.to_uppercase();

    if upper.contains("ALL") {
        return Some(HashSet::from(DayOfWeek::all()));
    }
    
    Some(
        HashSet::from_iter(
            upper.chars().filter_map(|c| parse_dayofweek(c))
        )
    )
}

fn parse_repeat_at(token: &String) -> Option<i32> {
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

fn parse_repeat_weekly(token: &String) -> bool {
    token.to_lowercase().contains("rep")
}

/// Returns a tuple of (remind_at, on_days, repeat_weekly)
pub fn parse_time_string(time_str: String) -> Option<(i32, HashSet<DayOfWeek>, bool)> {
    let tokens: Vec<String> = time_str.split(" ").map(String::from).collect();

    if tokens.len() < 2 {
        return None;
    }

    Some(
        (
            parse_repeat_at(&tokens[0])?, 
            parse_on_days(&tokens[1])?, 
            parse_repeat_weekly(&tokens.get(2).unwrap_or(&String::new()))
        )
    )
}