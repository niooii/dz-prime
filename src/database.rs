use serenity::all::UserId;
use sqlx::{postgres::PgPool, query, query_as};
use anyhow::Result;

use crate::model::{DayOfWeek, Task, TaskCreateInfo, TaskRow, UserSettings, UserSettingsRow};

pub struct Database {
    pool: PgPool
}

impl Database {
    pub async fn new(postgres_url: &str) -> Result<Self> {
        Ok (Self {
            pool: PgPool::connect(postgres_url).await?
        })
    }

    pub async fn settings(&self, user_id: UserId) -> Result<UserSettings> {
        Ok(
            UserSettings::from_row_struct(
                query_as!(
                    UserSettingsRow,
                    r"SELECT * FROM settings
                    where user_id = $1
                    ",
                    user_id.to_string()
                ).fetch_one(&self.pool).await?
            )?
        )
    }

    pub async fn set_settings(&self, user_id: UserId, user_settings: UserSettings) -> Result<()> {
        query!(
            r"INSERT INTO settings (user_id, ack_phrase)
            VALUES ($1, $2)
            ON CONFLICT (user_id)
            DO UPDATE SET
            ack_phrase = EXCLUDED.ack_phrase;",
            user_id.to_string(),
            user_settings.ack_phrase
        ).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn task(&self, id: i64) -> Result<Task> {
        Ok(
            Task::from_row_struct(
                query_as!(
                    TaskRow,
                    r"SELECT * FROM tasks
                    where id = $1
                    ",
                    id
                ).fetch_one(&self.pool).await?
            )?
        )
    }

    pub async fn add_task(&self, user_id: UserId, task: &TaskCreateInfo) -> Result<Task> {
        // the things we do for type safety.
        let on_days: Vec<i32> = 
            task.on_days.iter().map(|e| i32::from(e.clone())).collect::<Vec<_>>();
        Ok(
            Task::from_row_struct(
                query_as!(
                    TaskRow,
                    r#"INSERT INTO tasks (user_id, title, info, remind_at, on_days, repeat_weekly)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    RETURNING *"#,
                    user_id.to_string(),
                    task.title,
                    task.info,
                    task.remind_at,
                    &on_days,
                    task.repeat_weekly
                ).fetch_one(&self.pool).await?
            )?
        )
    }

    pub async fn tasks_for(&self, user_id: UserId) -> Result<Vec<Task>> {
        Ok(
            query_as!(
                TaskRow,
                r"SELECT * FROM tasks
                where user_id = $1
                ",
                user_id.to_string()
            ).fetch_all(&self.pool).await?
            .into_iter().map(|t| Task::from_row_struct(t)).collect::<Result<Vec<Task>>>()?
        )
    }

    pub async fn all_tasks(&self) -> Result<Vec<Task>> {
        Ok(
            query_as!(
                TaskRow,
                r"SELECT * FROM tasks
                ",
            ).fetch_all(&self.pool).await?
            .into_iter().map(|t| Task::from_row_struct(t)).collect::<Result<Vec<Task>>>()?
        )
    }
}