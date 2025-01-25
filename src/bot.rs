use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use crate::database::Database;
use crate::model::{DayOfWeek, Task, TaskCreateInfo, TaskRemindInfo};
use crate::scheduler::{ScheduledTaskController, TaskScheduler};
use crate::time_parse::parse_time_string;
use serenity::all::{ChannelId, Colour, CreateEmbed, CreateMessage, Http, Mention, Ready, UserId};
use serenity::async_trait;
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

pub struct DZBot {
    db: Database,
    scheduler: TaskScheduler,
    controllers: RwLock<HashMap<UserId, Vec<ScheduledTaskController>>>
}

impl DZBot {
    pub async fn new(db: Database) -> Self {
        Self {
            db,
            scheduler: TaskScheduler::new().await.expect("Failed to run task scheduler"),
            controllers: RwLock::new(HashMap::new())
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

pub async fn spam_routine(
    http: Arc<Http>, 
    task_info: TaskRemindInfo, 
    from_controller: Arc<Mutex<watch::Receiver<bool>>>,
    to_controller: Arc<watch::Sender<bool>>,
    num_active: Arc<RwLock<u32>>
) {
    println!("START SPAM ROUTINE..");
    let embed = CreateEmbed::new()
        .title(task_info.title)
        .description(task_info.info)
        .color(Colour::from_rgb(255, 255, 255));
    let ping = CreateMessage::new()
        .content(format!("{} hey", task_info.user_id.mention().to_string()));
    let mut channel = task_info.user_id.create_dm_channel(http.clone())
    .await;

    while let Err(e) = channel {
        eprintln!("Error fetching channel: {e}.");
        let retry_time = 300;
        eprintln!("Trying to refetch in {retry_time}ms..");
        time::sleep(Duration::from_millis(retry_time)).await;
        channel = task_info.user_id.create_dm_channel(http.clone())
            .await;
    }

    let channel = channel.expect("tf??");
    println!("Fetched channel.");

    // Signal routine start
    to_controller.send(true).expect("Failed to send message to controller..");

    channel.send_message(http.clone(), CreateMessage::new().embed(embed))
        .await.expect("failed to send initial embed gg");

    loop {
        // ping ping ping 
        let ghost_ping = || async {
            let ping_msg = channel.send_message(http.clone(), ping.clone())
                .await.expect("Failed to send ping");
            let _ = ping_msg.delete(http.clone()).await;
        };
        ghost_ping().await;

        // either sleep for 2/3 seconds or break when sent signal
        let mut fc = from_controller.lock().await; 
        let num_active: u32 = {
            let lock = num_active.read().await;
            *lock
        };
        tokio::select! {
            // lets NOT get rate limited guys
            _ = time::sleep(Duration::from_secs_f32(0.5 * num_active as f32)) => continue,
            _ = fc.changed() => break
        }
    }

    to_controller.send(false).expect("Failed to send message to controller..");
    println!("spam routine end..");
}

impl DZBot {
    async fn add_controller(&self, uid: UserId, c: ScheduledTaskController) {
        let mut controllers = self.controllers.write().await;
        let tasks = controllers.entry(uid)
            .or_insert_with(Vec::new);
        tasks.push(c);
    }
}

#[async_trait]
impl EventHandler for DZBot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("bot started!");

        let tasks = self.db.all_tasks().await.expect("Could not get all tasks");
        println!("found {} tasks.. rescheduling all...", tasks.len());

        let mut controllers = self.controllers.write().await;
        for task in tasks {
            let uid = match task {
                Task::Recurring { user_id, .. } => user_id,
                Task::Once { user_id, .. } => user_id,
            };
            let v = controllers.entry(uid)
            .or_insert_with(Vec::new);

            v.push(
                self.scheduler.add_task(ctx.http.clone(), task)
                    .await.expect("Failed to add task")
            );
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
            let mut controllers = self.controllers.write().await;
            let tasks = controllers
                .get_mut(&uid);
            let mut stopped_task = false;
            if let Some(tasks) = tasks {
                // TODO! task and associated controller removal
                for t in tasks {
                    if t.running() {
                        t.stop().await.expect("Failed to call stop");
                        stopped_task = true;
                    }
                }
            } 
            if stopped_task {
                return;
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
        let task = match self.db.add_task(msg.author.id, &create_info).await {
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
                self.add_controller(msg.author.id, c).await;
            }
        }

        msg.reply_ping(ctx, "i gotchu").await
            .expect("couldnt alert user of SUCCESS??");
    }
}
