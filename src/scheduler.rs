use serenity::all::Http;
use tokio::sync::{oneshot, watch};
use tokio_cron_scheduler::{JobScheduler, Job};
use chrono::{Duration, Local, Offset};
use std::{sync::Arc, time::Duration as StdDuration};
use anyhow::Result;

use crate::{bot::spam_routine, model::{Task, TaskRemindInfo}};

pub struct TaskScheduler {
    scheduler: Arc<JobScheduler>
}

impl TaskScheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        let scheduler = Arc::new(scheduler);
        
        scheduler.start().await?;
        
        Ok(TaskScheduler {
            scheduler,
        })
    }

    // Returns a channel.
    // Upon sending any value to this channel, if active, the spam routine will stop.
    pub async fn add_task(&self, http: Arc<Http>, task: Task) -> Result<ScheduledTaskController> {
        // account for UTC offset arrhghghhg
        let utc_offset = chrono::Local::now()
            .offset().fix().local_minus_utc() / 60;

        println!("Adding task: {task:?}");
        let minute = (task.remind_at - utc_offset) % 1440;
        let days_str = task.on_days.clone().into_iter().map(|d| i32::from(d).to_string())
            .collect::<Vec<String>>().join(",");
        println!("day str: {}", days_str);
        let cron = format!("0 {} {} * * {}", minute % 60, minute / 60, days_str);

        let (to_scheduled, from_controller) = watch::channel(false);
        let (to_controller, from_scheduled) = watch::channel(false);
        let fc = Arc::new(from_controller);
        let tc = Arc::new(to_controller);
        let job = Job::new_async(&cron, 
            move |_uuid, _lock| {
            let fc = fc.clone();
            let tc = tc.clone();
            let http = http.clone();
            let task_info = TaskRemindInfo {
                title: task.title.clone(),
                info: task.info.clone(),
                user_id: task.user_id.clone()
            };
            Box::pin(async move {
                spam_routine(http, task_info, fc, tc).await;
            })
        })?;
        
        self.scheduler.add(job).await?;
        
        Ok(
            ScheduledTaskController {
                to_scheduled,
                from_scheduled
            }
        )
    }
}

pub struct ScheduledTaskController {
    // to send a bool to the scheduled function and stop it
    to_scheduled: watch::Sender<bool>,
    // to recieve a bool from the scheduled function to know if it started running
    from_scheduled: watch::Receiver<bool>
}

impl ScheduledTaskController {
    pub fn running(&self) -> bool {
        *self.from_scheduled.borrow()
    }

    pub async fn stop(&self) -> Result<()> {
        // stop routine
        self.to_scheduled.send(true)?;
        
        // wait for a false value to be sent
        while *self.from_scheduled.borrow() {
            tokio::time::sleep(std::time::Duration::from_secs_f32(0.1)).await;
        }
        // then reset
        self.to_scheduled.send(false)?;
        Ok(())
    }
}