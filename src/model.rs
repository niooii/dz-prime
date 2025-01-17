use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub struct UserSettings {
    pub should_dm: bool,
    pub ack_phrase: String
}

#[derive(sqlx::FromRow)]
struct Task {
    id: i32,
    user_id: String,
    title: String,
    info: String,
    remind_at: i32,
    on_days: Vec<i32>, 
    repeat_weekly: bool,
}

impl Task {
    
}