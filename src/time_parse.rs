use std::collections::HashSet;
use chrono::{NaiveTime, Timelike};
use time::{macros::format_description, Date, Time, Weekday};

fn parse_dayofweek(c: char) -> Option<Weekday> {
    match c {
        'U' => Some(Weekday::Sunday),
        'M' => Some(Weekday::Monday),
        'T' => Some(Weekday::Tuesday), 
        'W' => Some(Weekday::Wednesday),
        'R' => Some(Weekday::Thursday),
        'F' => Some(Weekday::Friday),
        'S' => Some(Weekday::Saturday),
        _ => None
    }
}

fn parse_on_days(token: &String) -> Result<HashSet<Weekday>, String> {
    if token.is_empty() {
        return Err("no days of week?".into());
    }
    let upper = token.to_uppercase();

    if upper.contains("A") {
        return Ok(HashSet::from(
            [
                Weekday::Sunday,
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
                Weekday::Saturday,
            ]
        ));
    }

    let set = HashSet::from_iter(
        upper.chars().filter_map(parse_dayofweek)
    );

    if set.is_empty() {
        return Err("no days of week?".into());
    }
    
    Ok(set)
}

fn parse_repeat_at(token: &String) -> Result<Time, String> {
    let upper = token.to_uppercase();
    let parsers = [
        format_description!("[hour repr:12 padding:none]:[minute][period]"),
        format_description!("[hour repr:12 padding:none]:[minute] [period]"),
        format_description!("[hour repr:12 padding:none][period]"),
        format_description!("[hour padding:none]:[minute]"),
    ];
    for parser in parsers {
        match Time::parse(&upper, parser) {
            Err(e) => {
                eprintln!("trying to parse again due to err {e}");
            },
            Ok(t) => return Ok(t)
        }
    }
    Err("could not parse time".into())
}

fn parse_repeat_weekly(token: &String) -> bool {
    token.to_lowercase().contains("rep")
}

pub struct TimeParseResult {
    remind_time: Time,
    days_of_week: HashSet<Weekday>,
    repeat_weekly: bool,
    date: Date
}

/// Returns a tuple of (remind_at, on_days, repeat_weekly)
pub fn parse_time_string(time_str: &str) -> Result<TimeParseResult, String> {
    let tokens: Vec<String> = time_str.split(" ").map(String::from).collect();

    if tokens.len() < 2 {
        return Err("are you stupid you must be stupid".into());
    }

    // find the first string with numbers in it LOL (and doesnt contain '/' because reserved for date)
    let time_token = tokens.iter()
        .filter(|s| 
            !s.contains("/") && s.chars().find(|c| char::is_numeric(*c)).is_some()
        )
        .next()
        .ok_or("no time?".to_string())?;

    let date_token = tokens.iter()
        .filter(|s| 
            !s.contains("/") && s.chars().find(|c| char::is_numeric(*c)).is_some()
        )
        .next().unwrap_or(&String::new());

    Ok(
        TimeParseResult {
            remind_time: parse_repeat_at(time_token)?, 
            days_of_week: parse_on_days(&tokens[1])?, 
            repeat_weekly: parse_repeat_weekly(&tokens.get(2).unwrap_or(&String::new()))
        }
    )
}