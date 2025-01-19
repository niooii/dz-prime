use serenity::all::Http;
use tokio::sync::{oneshot, watch};
use tokio_cron_scheduler::{JobScheduler, Job};
use chrono::{Duration, Local, Offset};
use std::{sync::Arc, time::Duration as StdDuration};
use anyhow::Result;

use crate::{bot::spam_routine, model::Task};

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
    pub async fn add_task(&self, http: Arc<Http>, task: Task) -> Result<watch::Sender<bool>> {
        // account for UTC offset arrhghghhg
        let utc_offset = chrono::Local::now()
            .offset().fix().local_minus_utc() / 60;

        println!("Adding task: {task:?}");
        let minute = (task.remind_at - utc_offset) % 1440;
        let days_str = task.on_days.clone().into_iter().map(|d| i32::from(d).to_string())
            .collect::<Vec<String>>().join(",");
        println!("day str: {}", days_str);
        let cron = format!("0 {} {} * * {}", minute % 60, minute / 60, days_str);

        let (tx, rx) = watch::channel(false);
        let rx = Arc::new(rx);
        let job = Job::new_async(&cron, move |_uuid, _lock| {
            let rx = rx.clone();
            Box::pin(async move {
                spam_routine(rx).await;
            })
        })?;
        
        self.scheduler.add(job).await?;
        
        Ok(tx)
    }
}