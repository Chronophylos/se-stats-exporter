#![warn(missing_copy_implementations, missing_debug_implementations)]

use clap::arg_enum;
use metrics::{gauge, IntoLabels};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use stats_api::{ApiClient, ChatterStats};
use tracing::{debug, error, instrument};

pub mod stats_api;

arg_enum! {
    #[derive(PartialEq, Debug)]
    pub enum ExportName {
        Bttv,
        Ffz,
        Twitch,
        Hashtag,
        Command,
        Chatter,
        Channel,
        TotalMessages,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExportConfig {
    bttv: bool,
    ffz: bool,
    twitch: bool,
    hashtag: bool,
    command: bool,
    chatter: bool,
    channel: bool,
    total_messages: bool,
}

impl ExportConfig {
    pub fn all() -> Self {
        Self {
            bttv: true,
            ffz: true,
            twitch: true,
            hashtag: true,
            command: true,
            chatter: true,
            channel: true,
            total_messages: true,
        }
    }
}

impl From<Vec<ExportName>> for ExportConfig {
    fn from(values: Vec<ExportName>) -> Self {
        let mut config = Self::default();

        for value in values {
            match value {
                ExportName::Bttv => config.bttv = true,
                ExportName::Ffz => config.ffz = true,
                ExportName::Twitch => config.twitch = true,
                ExportName::Hashtag => config.hashtag = true,
                ExportName::Command => config.command = true,
                ExportName::Chatter => config.chatter = true,
                ExportName::Channel => config.channel = true,
                ExportName::TotalMessages => config.total_messages = true,
            }
        }

        config
    }
}

fn drain_to_gauge<'a, I, L, ValueF, LabelF>(
    name: &'static str,
    data: I,
    value_f: ValueF,
    label_f: LabelF,
) where
    I: IntoIterator,
    ValueF: Fn(&I::Item) -> f64,
    LabelF: Fn(&I::Item) -> L,
    L: IntoLabels,
{
    data.into_iter()
        .for_each(|entry| gauge!(name, value_f(&entry), label_f(&entry)));
}

#[instrument(skip(client))]
pub async fn export_stats(config: &ExportConfig, client: &ApiClient) {
    let stats = match client.get_stats("global").await {
        Err(e) => {
            error!("Could not get stats from stats.streamelements.com: {}", e);
            return;
        }
        Ok(s) => s,
    };

    let top_channels = match client.get_top_channels().await {
        Err(e) => {
            error!(
                "Could not get top channels from stats.streamelements.com: {}",
                e
            );
            return;
        }
        Ok(s) => s,
    };

    debug!("Exporting stats to Prometheus");

    if config.total_messages {
        gauge!("sestats.total-messages", stats.total_messages as f64);
    }

    if config.chatter {
        // stats.chatters.into_par_iter().for_each(|chatter| {
        //     gauge!(
        //         "sestats.chatter",
        //         chatter.amount as f64,
        //         &[("name", chatter.name.to_string()),]
        //     )
        // });
        drain_to_gauge(
            "sestats.chatter",
            stats.chatters.to_vec(),
            |chatter: &ChatterStats| chatter.amount as f64,
            |chatter: &ChatterStats| &[("name", chatter.name.to_string())],
        )
    }

    if config.hashtag {
        stats.hashtags.into_par_iter().for_each(|hashtag| {
            gauge!(
                "sestats.hashtag",
                hashtag.amount as f64,
                &[("hashtag", hashtag.hashtag.to_string()),]
            )
        });
    }

    if config.command {
        stats.commands.into_par_iter().for_each(|command| {
            gauge!(
                "sestats.hashtag",
                command.amount as f64,
                &[("command", command.command.to_string()),]
            )
        });
    }

    if config.bttv {
        stats.bttv_emotes.into_par_iter().for_each(|emote| {
            gauge!(
                "sestats.emote",
                emote.amount as f64,
                &[
                    ("provider", String::from("bttv")),
                    ("emote", emote.emote.to_string()),
                ]
            )
        });
    }

    if config.ffz {
        stats.ffz_emotes.into_par_iter().for_each(|emote| {
            gauge!(
                "sestats.emote",
                emote.amount as f64,
                &[
                    ("provider", String::from("ffz")),
                    ("emote", emote.emote.to_string()),
                ]
            )
        });
    }

    if config.twitch {
        stats.twitch_emotes.into_par_iter().for_each(|emote| {
            gauge!(
                "sestats.emote",
                emote.amount as f64,
                &[
                    ("provider", String::from("twitch")),
                    ("emote", emote.emote.to_string()),
                ]
            )
        });
    }

    if config.channel {
        top_channels.into_par_iter().for_each(|channel| {
            gauge!(
                "sestats.channel",
                channel.messages as f64,
                &[("channel", channel.channel.to_string())]
            )
        });
    }

    debug!("Finished exporting stats")
}
