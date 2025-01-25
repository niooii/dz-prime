use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use crate::database::Database;
use crate::jobs::{EmbedReminderJob, SpamPingJob, SpamPingSignal, SpamPingStatus};
use crate::model::{DayOfWeek, Task, TaskCreateInfo, TaskRemindInfo};
use crate::scheduler::{TaskScheduler};
use crate::time_parse::parse_time_string;
use serenity::all::{Channel, ChannelId, Colour, CreateEmbed, CreateMessage, Http, Mention, MessageBuilder, ReactionType, Ready, UserId};
use serenity::{async_trait, json::json};
use serenity::model::channel::Message;
use serenity::prelude::*;
use anyhow::Result;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time;

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
9am a rep (a = ALL)
9:30am UMTWRFS
9:30am umtwrfs rep
10pm mwf
";

pub struct DzContextInner {
    pub db: Arc<Database>,
    pub spammer_ctl: HashMap<UserId, SpamPingJob>,
    // map of the reminder task ID and user id in the database to the job
    pub reminders_ctl: HashMap<i64, EmbedReminderJob>,
}

pub type DzContext = Arc<RwLock<DzContextInner>>;

pub struct DZBot {
    db: Arc<Database>,
    scheduler: TaskScheduler,
    ctx: DzContext
}

impl DzContextInner {
    /// Attempts to retrieve a channel from the database, otherwise uses discord's api
    pub async fn get_dm_channel(&self, http: Arc<Http>, uid: UserId) -> Result<ChannelId> {
        let ch_fetch = self.db.dm_channel(&uid)
            .await?;
        Ok(
            match ch_fetch {
                None => {
                    let mut channel = 
                        uid.create_dm_channel(http.clone())
                        .await;
                    
                    while let Err(e) = channel {
                        eprintln!("Error fetching channel: {e}.");
                        let retry_time = 300;
                        eprintln!("Trying to refetch in {retry_time}ms..");
                        time::sleep(Duration::from_millis(retry_time)).await;
                        channel = uid.create_dm_channel(http.clone())
                            .await;
                    }
                    
                    let channel = channel.unwrap().id;
                    self.db.put_dm_channel(&uid, &channel).await?;
                    channel
                }
                Some(c) => c
            }
        )
    }
}

impl DZBot {
    pub async fn new(db: Arc<Database>) -> Self {
        let ctx = Arc::new(RwLock::new(
            DzContextInner {
                db: db.clone(),
                spammer_ctl: HashMap::new(),
                reminders_ctl: HashMap::new()
            }
        ));
        Self {
            scheduler: TaskScheduler::new(ctx.clone()).expect("Failed to run task scheduler"),
            ctx,
            db,
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

async fn report_err(channel: ChannelId, http: Arc<Http>, err: impl ToString + Into<String>) {
    let res = channel.send_message(
        http, 
        CreateMessage::new().content(err)
    ).await;

    if let Err(e) = res {
        eprintln!("Failed to log err to user: {e}");
    }
}

#[async_trait]
impl EventHandler for DZBot {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        println!("bot started!");

        let tasks = self.db.all_tasks().await.expect("Could not get all tasks");
        println!("found {} tasks.. rescheduling all...", tasks.len());

        for task in tasks {
            self.scheduler.add_task(ctx.http.clone(), task).await.unwrap();
        }

        println!("finished rescheduling all tasks...");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        
        // If messaged in a guild, check if the channel id is the one specified 
        if msg.guild_id.is_some() {
            return;
        }
        
        // Check if user is tryna stop a mass pinging
        {
            let uid = msg.author.id;
            let dzctx = self.ctx.read().await;
            let spam_job = dzctx.spammer_ctl
                .get(&uid);

            if let Some(s) = spam_job {
                if s.status() == SpamPingStatus::Active {
                    s.signal(SpamPingSignal::Stop);
                    let _ = msg.react(ctx.http(), ReactionType::Unicode("👍".into())).await;
                    return;
                }
            } 
        }

        // Otherwise go on
        let create_info = match parse_text(&msg.content) {
            Ok(r) => r,
            Err(err_string) => {
                // might be:
                // HELP, TASKS, DELETE
                // TODO!
                let needs_a_hero = msg.content.to_lowercase().contains("help");
                if let Err(e) = msg.reply_ping(ctx, if needs_a_hero {HELP_STR} else {&err_string}).await {
                    eprintln!("{e}");
                }
                return;
            }
        };

        if create_info.on_days.len() == 0 {
            msg.reply_ping(ctx, String::from("enter some days man")).await
                .expect("couldnt alert user of failure");
            return;
        }

        // peak error management?
        let task = match self.db.add_task(&msg.author.id, &create_info).await {
            Ok(t) => t,
            Err(e) => {
                msg.reply_ping(ctx, format!("Failed to save task to db: {e}")).await
                    .expect("couldnt alert user of failure");
                return;
            }
        };
        match self.scheduler.add_task(ctx.http.clone(), task).await {
            Err(e) => {
                msg.reply_ping(ctx, format!("Failed to schedule task: {e}")).await
                    .expect("couldnt alert user of failure");
                return;
            }
            Ok(c) => {
                println!("success");
            }
        }

        msg.reply_ping(ctx, "ok").await
            .expect("couldnt alert user of SUCCESS??");
    }
}
