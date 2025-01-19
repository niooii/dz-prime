use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::database::Database;
use crate::model::{DayOfWeek, Task, TaskCreateInfo};
use crate::scheduler::TaskScheduler;
use crate::time_parse::parse_time_string;
use serenity::all::{ChannelId, Ready, UserId};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use anyhow::Result;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

const HELP_STR: &str = "
FORMAT EXAMPLE:
[TITLE]
[info...]
...
[TIME]

TIME is:
[time days repeatweekly]
VALID TIME EXAMPLES:
9am UMTWRFS rep
9am all rep
9:30am UMTWRFS
9:30am umtwrfs rep
10pm mwf
";

pub struct DZBot {
    db: Database,
    allowed_channels: Vec<ChannelId>,
    scheduler: TaskScheduler
}

impl DZBot {
    pub async fn new(db: Database) -> Self {
        Self {
            db,
            // TODO! kill with fire
            allowed_channels: vec![
                ChannelId::new(1329597532317417583)
            ],
            scheduler: TaskScheduler::new().await.expect("Failed to run task scheduler")
        }
    }
}

fn parse_text(content: &String) -> Result<TaskCreateInfo, String> {
    let mut lines = content.lines();
    let title = lines.next().ok_or(String::from("no title?"))?.to_string();
    let times_str = lines.next_back().ok_or(String::from("no times?"))?.to_string();
    let (remind_at, on_days, repeat_weekly) = 
        parse_time_string(times_str).ok_or(String::from("bad time"))?;
    let info: String = lines.collect::<Vec<_>>().join("\n");

    Ok(
        TaskCreateInfo {
            title,
            info,
            on_days,
            remind_at,
            repeat_weekly
        }
    )
}

pub async fn spam_routine(rx: Arc<watch::Receiver<bool>>) {
    while !*rx.borrow() {

        // ping ping ping 

        tokio::select! {
            _ = rx.changed() => {
                if *rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs_f32(0.66)) => {}
        }
    }
}

#[async_trait]
impl EventHandler for DZBot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("bot started!");

        let tasks = self.db.all_tasks().await.expect("Could not get all tasks");
        println!("found {} tasks.. rescheduling all...", tasks.len());
        
        for task in tasks {
            self.scheduler.add_task(ctx.http.clone(), task).await.expect("Failed to add task");
        }
        println!("finished rescheduling all tasks...");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        
        // If messaged in a guild, check if the channel id is the one specified 
        if msg.guild_id.is_some() && !self.allowed_channels.contains(&msg.channel_id) {
            return;
        }

        let create_info = match parse_text(&msg.content) {
            Ok(r) => r,
            Err(err_string) => {
                let needs_a_hero = msg.content.to_lowercase().contains("help");
                if let Err(e) = msg.reply_ping(ctx, if needs_a_hero {HELP_STR} else {&err_string}).await {
                    eprintln!("{e}");
                }
                return;
            }
        };

        // peak error management?
        let task = match self.db.add_task(msg.author.id, &create_info).await {
            Ok(t) => t,
            Err(e) => {
                msg.reply_ping(ctx, format!("Failed to save task to db: {e}")).await
                .expect("couldnt alert user of failure");
                return;
            }
        };
        if let Err(e) = self.scheduler.add_task(ctx.http.clone(), task).await {
            msg.reply_ping(ctx, format!("Failed to schedule task: {e}")).await
                .expect("couldnt alert user of failure");
            return;
        }

        msg.reply_ping(ctx, "i gotchu").await
            .expect("couldnt alert user of SUCCESS??");
    }
}
