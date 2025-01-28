use std::{collections::HashSet, sync::Arc, time::Duration};

use chrono::Offset;
use itertools::Itertools;
use serenity::all::{ChannelId, Colour, CreateEmbed, CreateMessage, Http, Mentionable, UserId};
use ::time::{Date, OffsetDateTime, Weekday};
use tokio::{sync::{watch, Mutex}, time::{self, Instant, Sleep}};
use anyhow::Result;

use crate::{bot::DzContext, database::Database, model::{Task, TaskRemindInfo}};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SpamPingSignal {
    Start,
    Stop,
    End
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SpamPingStatus {
    Active,
    Stopped,
}

pub struct SpamPingJob {
    to_task: watch::Sender<SpamPingSignal>,
    from_task: watch::Receiver<SpamPingStatus>,
}

impl SpamPingJob {
    pub fn new(ctx: DzContext, http: Arc<Http>, user_id: UserId) -> Self {
        let (to_task, mut from_ctl) = watch::channel(SpamPingSignal::Stop);
        let (to_ctl, from_task) = watch::channel(SpamPingStatus::Stopped);
        
        tokio::spawn(async move {
            let channel = 
                ctx.read().await.get_dm_channel(http.clone(), user_id).await
                .unwrap();
            let ping = CreateMessage::new()
                .content(format!("{} hey buddy", user_id.mention()));
            'outer: loop {
                if let Err(e) = from_ctl.changed().await {
                    // channel closes here
                    eprintln!("gg: {e}");
                    return;
                }
                'inner: loop {
                    let val = *from_ctl.borrow_and_update();
                    match val {
                        SpamPingSignal::Start => {
                            to_ctl.send(SpamPingStatus::Active);
                            // ghost ping user
                            let msg = channel.send_message(http.clone(), ping.clone())
                                .await.expect("Failed to send message to user");
                            let _ = msg.delete(http.clone()).await;

                            tokio::select! {
                                _ = time::sleep(Duration::from_secs_f32(1.5)) => 
                                    continue 'inner,
                                _ = from_ctl.changed() => continue 'inner
                            };
                        },
                        SpamPingSignal::Stop => {
                            to_ctl.send(SpamPingStatus::Stopped);
                            break 'inner
                        },
                        SpamPingSignal::End => {
                            to_ctl.send(SpamPingStatus::Stopped);
                            break 'outer
                        },
                    }
                }
                println!("stop.");
            }
        });

        SpamPingJob {
            to_task,
            from_task
        }
    }

    pub fn status(&self) -> SpamPingStatus {
        *self.from_task.borrow()
    }

    pub fn signal(&self, signal: SpamPingSignal) {
        self.to_task.send(signal).unwrap();
    }
}

pub struct EmbedReminderJob {
    // would use a oneshot but might add more states later
    to_task: watch::Sender<bool>,
    from_task: Mutex<watch::Receiver<bool>>,
}

impl EmbedReminderJob {
    pub fn new(ctx: DzContext, http: Arc<Http>, task: &Task) -> Self {
        let (to_task, mut from_ctl) = watch::channel(false);
        let (_to_ctl, from_task) = watch::channel(false);

        tokio::spawn(embed_reminder_job(ctx, http, task.clone(), from_ctl));

        EmbedReminderJob {
            to_task,
            from_task: Mutex::new(from_task)
        }
    }

    /// Immediately stops the scheduled reminder task
    pub fn kill(&self) -> Result<()> {
        self.to_task.send(true)
            .map_err(anyhow::Error::from)
    }
}

async fn embed_reminder_job(
    ctx: DzContext,
    http: Arc<Http>,
    task: Task,
    mut from_ctl: watch::Receiver<bool>,
) {
    let task_info = task.remind_info();
    let id = task.id();

    let remove = || async {
        let mut ctx = ctx.write().await;
        ctx.reminders_ctl.remove_entry(&id);
        ctx.db.delete_task(id).await.expect("Could not delte for some reason");
    };

    loop {
        if let Some(sleep) = sleep_until_next(&task) {
            tokio::select! {
                _ = sleep => {
                    // do nothing and continue
                },
                _ = from_ctl.changed() => {
                    // this means a cancel signal has been sent.
                    remove().await;
                    return;
                }
            };
        } else {
            // theres no more times to repeat this task
            // remove remind task
            remove().await;
            // kill this thread
            return;
        }

        send_embed(
            http.clone(), 
            ctx.clone(),
            task_info.clone(), 
        ).await.unwrap();

        let m = ctx.read().await;
        let ctl = m.spammer_ctl.get(&task_info.user_id).unwrap();
        ctl.signal(SpamPingSignal::Start);
    }
}

/// Returns the next occurence or None if there isnt one.
pub fn next_occurrence_time(task: &Task) -> Option<OffsetDateTime> {
    let now = OffsetDateTime::now_utc();
    match task {
        Task::Once { remind_at, date, .. } => {
            let dt = date.with_time(*remind_at).assume_utc();
            (dt > now).then_some(dt)
        }
        Task::Recurring { remind_at, on_days, repeat_weekly, created_at, .. } => {
            // use the previous day as the referece point for date.next_occurence(Weekday), because the current day can count as well.
            let ref_date = created_at.date().previous_day().unwrap();

            if *repeat_weekly {
                let closest = on_days.iter()
                    .map(|d| {
                        let dt = ref_date.next_occurrence(*d).with_time(*remind_at).assume_utc();
                        if dt <= now {
                            ref_date.nth_next_occurrence(*d, 2).with_time(*remind_at).assume_utc()
                        } else {
                            dt
                        }
                    })
                    .sorted().next()?;

                Some(closest)
            } else {
                let closest = on_days.iter()
                    .map(|d| ref_date.next_occurrence(*d).with_time(*remind_at).assume_utc())
                    .filter(|d| *d > now)
                    .sorted().next()?;

                let days_since_created = (closest-*created_at).whole_days();
                (days_since_created < 7).then_some(closest)
            }
        }
    }
}

/// Returns None if there is no next occurrence
fn sleep_until_next(task: &Task) -> Option<Sleep> {
    let next = next_occurrence_time(task)?;
    println!("next occurence time: {next}");
    let instant = Instant::now();
    let now = OffsetDateTime::now_utc();
    
    assert!(next >= now, "gg cooked");

    let dur = next - now;
    let dur = std::time::Duration::new(
        dur.whole_seconds() as u64,
        dur.subsec_nanoseconds() as u32
    );

    Some(time::sleep_until(instant + dur))
}

async fn send_embed(
    http: Arc<Http>,
    ctx: DzContext, 
    task_info: TaskRemindInfo, 
) -> Result<()> {
    let embed = CreateEmbed::new()
        .title(task_info.title)
        .description(task_info.info)
        .color(Colour::from_rgb(255, 255, 255));

    let channel = ctx.read().await.get_dm_channel(http.clone(), task_info.user_id)
        .await?;

    channel.send_message(http.clone(), CreateMessage::new().embed(embed))
        .await?;

    Ok(())
}
