use std::collections::HashSet;

use crate::database::Database;
use crate::model::{Task, TaskCreateInfo};
use crate::time_parse::parse_time_string;
use serenity::all::{ChannelId, UserId};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use anyhow::Result;

const HELP_STR: &str = "
FORMAT EXAMPLE:
[TITLE]
[info...]
...
[TIME]

VALID TIME EXAMPLES:
9am UMTWRFS rep
9am all rep
9:30am UMTWRFS
9:30am umtwrfs rep
10pm mwf
";

pub struct DZBot {
    db: Database,
    allowed_channels: Vec<ChannelId>
}

impl DZBot {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            // TODO! kill with fire
            allowed_channels: vec![
                ChannelId::new(1329597532317417583)
            ]
        }
    }
}

fn parse_text(content: &String) -> Result<TaskCreateInfo, String> {
    let mut lines = content.lines();
    let title = lines.next().ok_or(String::from("no title?"))?.to_string();
    let times_str = lines.next_back().ok_or(String::from("no times?"))?.to_string();
    let (remind_at, on_days, repeat_weekly) = parse_time_string(times_str);
    let info: String = lines.collect::<Vec<_>>().join("\n");

    // TODO! parse time and day
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

#[async_trait]
impl EventHandler for DZBot {
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
                if let Err(e) = msg.reply(ctx, if needs_a_hero {HELP_STR} else {&err_string}).await {
                    eprintln!("{e}");
                }
                return;
            }
        };

        self.db.add_task(msg.author.id, &create_info).await.expect("bnlah");

        println!("{:?}", self.db.all_tasks().await);
    }
}
