use serenity::all::Http;
use tokio::{sync::{oneshot, watch}, task::JoinHandle};
use chrono::{Duration, Local, Offset};
use std::{sync::Arc, time::Duration as StdDuration};
use anyhow::Result;

use crate::{bot::spam_routine, model::{Task, TaskRemindInfo}};

pub struct TaskScheduler;

async fn job(
    http: Arc<Http>, 
    from_controller: Arc<watch::Receiver<bool>>,
    to_controller: Arc<watch::Sender<bool>>,
    task: Task
) {
    let (task_info, remind_at) = match &task {
        Task::Recurring { user_id, title, info, remind_at, .. } => 
        (
            TaskRemindInfo {
                title: title.into(),
                info: info.into(),
                user_id: user_id.clone(),
            },
            remind_at
        ),
        Task::Once { user_id, title, info, remind_at, .. } => 
        (
            TaskRemindInfo {
                title: title.into(),
                info: info.into(),
                user_id: user_id.clone(),
            },
            remind_at
        ),
    };
    // calculate wait time
    // account for UTC offset arrhghghhg
    let utc_offset = chrono::Local::now()
        .offset().fix().local_minus_utc() / 60;

    let minute = (remind_at - utc_offset) % 1440;
    // tokio::time::sleep_until()
    loop {
        spam_routine(
            http.clone(), 
            task_info.clone(), 
            from_controller.clone(), 
            to_controller.clone()
        ).await;
        tokio::time::sleep(std::time::Duration::from_secs_f32(2.0)).await;
    }
}

impl TaskScheduler {
    pub async fn new() -> Result<Self> {
        Ok(TaskScheduler)
    }

    // Returns a channel.
    // Upon sending any value to this channel, if active, the spam routine will stop.
    pub async fn add_task(&self, http: Arc<Http>, task: Task) -> Result<ScheduledTaskController> {
        
        // let days_str = task.on_days.clone().into_iter().map(|d| i32::from(d).to_string())
        //     .collect::<Vec<String>>().join(",");
        // println!("day str: {}", days_str);
        // let cron = format!("0 {} {} * * {}", minute % 60, minute / 60, days_str);

        let (to_scheduled, from_controller) = watch::channel(false);
        let (to_controller, from_scheduled) = watch::channel(false);
        let fc = Arc::new(from_controller);
        let tc = Arc::new(to_controller);

        println!("Adding task: {task:?}");
        
        let task_handle = tokio::task::spawn(
            job(http, fc, tc, task)
        );
        
        Ok(
            ScheduledTaskController {
                to_scheduled,
                from_scheduled,
                task_handle
            }
        )
    }
}

pub struct ScheduledTaskController {
    // to send a bool to the scheduled function and stop it
    to_scheduled: watch::Sender<bool>,
    // to recieve a bool from the scheduled function to know if it started running
    from_scheduled: watch::Receiver<bool>,
    task_handle: JoinHandle<()>
}

impl ScheduledTaskController {
    pub fn running(&self) -> bool {
        *self.from_scheduled.borrow()
    }

    pub async fn stop(&mut self) -> Result<()> {
        // stop routine
        self.to_scheduled.send(true)?;
        self.from_scheduled.mark_unchanged();
        
        // wait for a false value to be sent
        self.from_scheduled.changed().await?;
        // then reset
        self.to_scheduled.send(false)?;
        Ok(())
    }
}