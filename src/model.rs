use std::{collections::HashSet, convert::{TryFrom, TryInto}, f32::consts::PI};
use std::iter::FromIterator;
use anyhow::Result;
use chrono::{Local, Offset};
use serenity::all::UserId;
use sqlx::{postgres::PgHasArrayType, types::time::{Date, OffsetDateTime}};
use time::{convert::Week, Time, Weekday};
use ::time::UtcOffset;

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
    pub remind_at: Time,
    pub on_days: Option<Vec<i32>>, 
    pub repeat_weekly: bool,
    pub time_created: OffsetDateTime,
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

#[derive(Debug, Clone)]
pub enum Task {
    Recurring {
        id: i64,
        user_id: UserId,
        title: String,
        info: String,
        remind_at: Time,
        on_days: HashSet<Weekday>, 
        repeat_weekly: bool,
        created_at: OffsetDateTime
    },
    Once {
        id: i64,
        user_id: UserId,
        title: String,
        info: String,
        remind_at: Time,
        date: Date,
        created_at: OffsetDateTime
    }
}

impl Task {
    pub fn from_row_struct(row: TaskRow) -> Result<Self> {
        let weekday_from_i32 = |i: &i32| {
            match i {
                1 => Weekday::Sunday,
                2 => Weekday::Monday,
                3 => Weekday::Tuesday,
                4 => Weekday::Wednesday,
                5 => Weekday::Thursday,
                6 => Weekday::Friday,
                7 => Weekday::Saturday,
                _ => panic!("Invalid weekday number: {}", i),
            }
        };
        Ok(
            if let Some(date) = row.on_date {
                Self::Once {
                    id: row.id,
                    user_id: UserId::new(row.user_id.parse::<u64>()?),
                    title: row.title,
                    info: row.info,
                    remind_at: row.remind_at,
                    date,
                    created_at: row.time_created
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
                            row.on_days.unwrap().iter().map(weekday_from_i32)
                        )
                    },
                    repeat_weekly: row.repeat_weekly,
                    created_at: row.time_created
                }
            }
        )
    }

    pub fn id(&self) -> i64 {
        match self {
            Self::Recurring { id, .. }
            | Self::Once { id, .. } => *id
        }
    }

    pub fn user_id(&self) -> &UserId {
        match self {
            Self::Recurring { user_id, .. }
            | Self::Once { user_id, .. } => user_id
        }
    }

    pub fn repeats_weekly(&self) -> bool {
        match self {
            Self::Recurring { repeat_weekly, .. } => *repeat_weekly,
            Self::Once { .. } => false
        }
    }

    pub fn remind_at(&self) -> Time {
        match self {
            Self::Recurring { remind_at, .. }
            | Self::Once { remind_at, .. } => *remind_at
        }
    }

    pub fn created_at(&self) -> &OffsetDateTime {
        match self {
            Self::Recurring { created_at, .. }
            | Self::Once { created_at, .. } => created_at
        }
    }

    pub fn recurring(&self) -> bool {
        match self {
            Self::Recurring {..} => true,
            Self::Once {..} => false
        }
    }

    pub fn remind_info(&self) -> TaskRemindInfo {
        match self {
            Self::Once { user_id, title, info, .. } | Task::Recurring { user_id, title, info, .. } => 
            TaskRemindInfo {
                title: title.into(),
                info: info.into(),
                user_id: *user_id,
            },
        }
    }

    pub fn on_days(&self) -> Option<&HashSet<Weekday>> {
    	match self {
    		Self::Once { .. } => None,
    		Self::Recurring { on_days, .. } => Some(on_days)
    	}
    }

	/// Local time
    pub fn datetime_local(&self) -> Option<OffsetDateTime> {
    	match self {
    		Self::Once { date, remind_at, .. } => {
                let offset_sec = Local::now()
                    .offset()
                    .local_minus_utc();
                let local_offset = UtcOffset::from_whole_seconds(offset_sec)
                    .expect("??");
    			Some(date.with_time(*remind_at)
                    .assume_offset(UtcOffset::UTC).to_offset(local_offset))
    		},
    		Self::Recurring { .. } => None
    	}
    }

	/// UTC time
    pub fn datetime_utc(&self) -> Option<OffsetDateTime> {
    	match self {
    		Self::Once { date, remind_at, .. } => {
    			Some(date.with_time(*remind_at).assume_offset(UtcOffset::UTC))
    		},
    		Self::Recurring { .. } => None
    	}
    }
}

pub struct TaskCreateInfo {
    pub title: String,
    pub info: String,
    pub remind_at: Time,
    pub date: Option<Date>,
    pub on_days: Option<HashSet<Weekday>>, 
    pub repeat_weekly: bool,
}

/// Contains all the necessary information for sending reminders.
#[derive(Clone)]
pub struct TaskRemindInfo {
    pub title: String,
    pub info: String,
    pub user_id: UserId
}