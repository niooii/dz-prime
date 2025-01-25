use std::{collections::HashSet, convert::{TryFrom, TryInto}};
use std::iter::FromIterator;
use anyhow::Result;
use serenity::all::UserId;
use sqlx::{postgres::PgHasArrayType, types::time::Date};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Hash)]
pub enum DayOfWeek {
    Sunday,     // 0
    Monday,     // 1
    Tuesday,    // 2
    Wednesday,  // 3
    Thursday,   // 4
    Friday,     // 5
    Saturday,   // 6
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid day number: {0}. Must be between 0 and 6")]
pub struct InvalidDayError(i32);

impl TryFrom<i32> for DayOfWeek {
    type Error = InvalidDayError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Sunday),
            1 => Ok(Self::Monday),
            2 => Ok(Self::Tuesday),
            3 => Ok(Self::Wednesday),
            4 => Ok(Self::Thursday),
            5 => Ok(Self::Friday),
            6 => Ok(Self::Saturday),
            invalid => Err(InvalidDayError(invalid)),
        }
    }
}

impl From<DayOfWeek> for i32 {
    fn from(day: DayOfWeek) -> i32 {
        match day {
            DayOfWeek::Sunday => 0,
            DayOfWeek::Monday => 1,
            DayOfWeek::Tuesday => 2,
            DayOfWeek::Wednesday => 3,
            DayOfWeek::Thursday => 4,
            DayOfWeek::Friday => 5,
            DayOfWeek::Saturday => 6,
        }
    }
}

impl DayOfWeek {
    pub fn all() -> [DayOfWeek; 7] {
        [
            Self::Sunday,
            Self::Monday,
            Self::Tuesday,
            Self::Wednesday,
            Self::Thursday,
            Self::Friday,
            Self::Saturday,
        ]
    }

    // Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }
}

/// Database row structs
#[derive(sqlx::FromRow)]
pub struct UserSettingsRow {
    pub user_id: String,
    pub ack_phrase: String
}

#[derive(sqlx::FromRow)]
pub struct TaskRow {
    pub id: i64,
    pub user_id: String,
    pub title: String,
    pub info: String,
    pub remind_at: i32,
    pub on_days: Option<Vec<i32>>, 
    pub repeat_weekly: bool,
    pub on_date: Option<Date>
}

/// Returned structs
#[derive(Debug)]

pub struct UserSettings {
    pub ack_phrase: String
}

impl UserSettings {
    pub fn from_row_struct(row: UserSettingsRow) -> Result<Self> {
        Ok(
            Self {
                ack_phrase: row.ack_phrase
            }
        )
    }
}

#[derive(Debug)]
pub enum Task {
    Recurring {
        id: i64,
        user_id: UserId,
        title: String,
        info: String,
        remind_at: i32,
        on_days: HashSet<DayOfWeek>, 
        repeat_weekly: bool,
    },
    Once {
        id: i64,
        user_id: UserId,
        title: String,
        info: String,
        remind_at: i32,
        date: Date
    }
}

impl Task {
    pub fn from_row_struct(row: TaskRow) -> Result<Self> {
        Ok(
            if let Some(date) = row.on_date {
                Self::Once {
                    id: row.id,
                    user_id: UserId::new(row.user_id.parse::<u64>()?),
                    title: row.title,
                    info: row.info,
                    remind_at: row.remind_at,
                    date
                }
            } else {
                Self::Recurring {
                    id: row.id,
                    user_id: UserId::new(row.user_id.parse::<u64>()?),
                    title: row.title,
                    info: row.info,
                    remind_at: row.remind_at,
                    // row.on_days should never be None bc input validation!
                    on_days: {
                        HashSet::from_iter(
                            row.on_days.unwrap().iter().map(|d| DayOfWeek::try_from(d.clone())
                                .expect("Day of week input should be sanitized. Wtf??"))
                        )
                    },
                    repeat_weekly: row.repeat_weekly
                }
            }
        )
    }
}

pub struct TaskCreateInfo {
    pub title: String,
    pub info: String,
    pub remind_at: i32,
    pub on_days: HashSet<DayOfWeek>, 
    pub repeat_weekly: bool,
}

/// Contains all the necessary information for sending reminders.
pub struct TaskRemindInfo {
    pub title: String,
    pub info: String,
    pub user_id: UserId
}