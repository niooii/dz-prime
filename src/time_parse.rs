use std::{collections::HashSet, u8};
use chrono::{Local, NaiveTime, Timelike};
use itertools::Itertools;
use time::{macros::format_description, Date, Month, OffsetDateTime, Time, UtcOffset, Weekday};

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

fn parse_on_days(token: &String) -> Option<HashSet<Weekday>> {
    if token.is_empty() {
        return None;
    }
    let upper = token.to_uppercase();

    if upper.contains("A") {
        return Some(HashSet::from(
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
        return None;
    }
    
    Some(set)
}

enum DayShift {
    Forward,
    Backward,
    None
}

/// Returns the parsed time IN UTC, and the day shift if the day got changed because of
/// timezone conversions.
fn parse_remind_at(token: &String) -> Result<(Time, DayShift), String> {
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
            Ok(t) => {
                let offset_sec = Local::now()
                    .offset()
                    .utc_minus_local();
                let local_offset = UtcOffset::from_whole_seconds(offset_sec)
                    .expect("??");
                let local_datetime = OffsetDateTime::now_utc();
                let utc_datetime = local_datetime
                    .replace_time(t).to_offset(local_offset);
                let day_shift = if utc_datetime.date() > local_datetime.date() {
                    DayShift::Forward
                } else if utc_datetime.date() < local_datetime.date() {
                    DayShift::Backward
                } else {
                    DayShift::None
                };
                return Ok((utc_datetime.time(), day_shift));
            }
        }
    }
    Err("could not parse time".into())
}

/// Time should be in UTC.
fn parse_date(token: &String, time: &Time, day_shift: DayShift) -> Option<Date> {
    let upper = token.to_uppercase();
    let (month, day) = if let Some(tup) = token.split('/')
    .map(|s| s.parse::<u8>().unwrap_or(u8::MAX))
    .collect_tuple::<(u8, u8)>() {
            tup
    } else {
        return None;
    };

    let now = OffsetDateTime::now_utc();
    let curr_year = now.year();
    Date::from_calendar_date(curr_year, Month::try_from(month).unwrap(), day)
    .map(|d| {
        // account for day going over bounds when converting from local to UTC.
        let date = match day_shift {
            DayShift::Forward => d.next_day().expect("the end of time"),
            DayShift::Backward => d.previous_day().expect("the end of time"),
            DayShift::None => d,
        };
        let date_time = date.with_time(*time).assume_offset(UtcOffset::UTC);
        if date_time <= now {
            // if the date is before rn, construct with next year
            date.replace_year(curr_year + 1).unwrap()
        } else {
            date
        }
    }).ok()
}

fn parse_repeat_weekly(token: &String) -> bool {
    token.to_lowercase().contains("rep")
}

pub struct TaskTimeInfo {
    pub remind_time: Time,
    pub days_of_week: Option<HashSet<Weekday>>,
    pub repeat_weekly: bool,
    pub date: Option<Date>
}

impl TaskTimeInfo {
    pub fn parse(str: &str) -> Result<Self, String> {
        let tokens: Vec<String> = str.split(" ").map(String::from).collect();

        if tokens.len() < 2 {
            return Err("are you stupid you must be stupid".into());
        }

        let (remind_time, day_shift) = parse_remind_at(&tokens[0])?;

        Ok(
            Self {
                // parse same token for date and days of week.
                days_of_week: parse_on_days(&tokens[1]), 
                date: parse_date(&tokens[1], &remind_time, day_shift),
                repeat_weekly: parse_repeat_weekly(&tokens.get(2).unwrap_or(&String::new())),
                remind_time 
            }
        )
    }
}