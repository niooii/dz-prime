use serenity::all::Http;
use tokio::{sync::{oneshot, watch, Mutex, RwLock}, task::JoinHandle};
use chrono::{Duration, Local, Offset};
use std::{sync::Arc, time::Duration as StdDuration};
use anyhow::Result;

use crate::{bot::DzContext, jobs::{EmbedReminderJob, SpamPingJob, SpamPingSignal}, model::{Task, TaskRemindInfo}};

pub struct TaskScheduler {
    ctx: DzContext
}

impl TaskScheduler {
    pub fn new(ctx: DzContext) -> Result<Self> {
        Ok(TaskScheduler { ctx })
    }

    // Returns a channel.
    // Upon sending any value to this channel, if active, the spam routine will stop.
    pub async fn add_task(&self, http: Arc<Http>, task: Task) -> Result<()> {
        
        // let days_str = task.on_days.clone().into_iter().map(|d| i32::from(d).to_string())
        //     .collect::<Vec<String>>().join(",");
        // println!("day str: {}", days_str);
        // let cron = format!("0 {} {} * * {}", minute % 60, minute / 60, days_str);

        // let (to_task, from_ctl) = watch::channel(false);
        // let (to_ctl, from_task) = watch::channel(false);

        println!("Adding task: {task:?}");
        let (task_id, uid) = match task {
            Task::Recurring { id, user_id, .. } => (id, user_id),
            Task::Once { id, user_id, .. } => (id, user_id),
        };

        let mut ctx = self.ctx.write().await;

        // insert a spammer controller if there isnt one
        ctx.spammer_ctl.entry(uid)
            .or_insert_with(|| {
                SpamPingJob::new(
                    self.ctx.clone(), 
                    http.clone(),
                    uid
                ).unwrap()
            });

        ctx.reminders_ctl
            .insert(
                task_id, 
                EmbedReminderJob::new(self.ctx.clone(), http, task)?
            );
        
        Ok(())
    }
}

