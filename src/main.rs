#![warn(missing_copy_implementations, missing_debug_implementations)]

use metrics::{gauge, register_gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::time;
use tracing::{debug, error};

mod stats_api;

const EXPORT_BTTV: bool = true;
const EXPORT_FFZ: bool = false;
const EXPORT_TWITCH: bool = false;
const EXPORT_HASHTAGS: bool = false;
const EXPORT_COMMANDS: bool = false;
const EXPORT_CHATTERS: bool = true;
const EXPORT_TOTAL_MESSAGES: bool = true;
const EXPORT_CHANNELS: bool = true;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    PrometheusBuilder::new()
        .listen_address(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            9001,
        ))
        .install()?;

    tracing_subscriber::fmt::init();

    register_gauge!("sestats.emote", "top emotes");
    register_gauge!("sestats.total-messages", "total messages on twitch");
    register_gauge!("sestats.chatter", "top chatters");
    register_gauge!("sestats.channel", "top channels");
    register_gauge!("sestats.command", "top commands");
    register_gauge!("sestats.hashtag", "top hashtags");

    let client = stats_api::ApiClient::new()?;

    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;

        let stats = match client.get_stats("global").await {
            Err(e) => {
                error!("Could not get stats from stats.streamelements.com: {}", e);
                continue;
            }
            Ok(s) => s,
        };

        let top_channels = match client.get_top_channels().await {
            Err(e) => {
                error!(
                    "Could not get top channels from stats.streamelements.com: {}",
                    e
                );
                continue;
            }
            Ok(s) => s,
        };

        debug!("Exporting stats to Prometheus");

        if EXPORT_TOTAL_MESSAGES {
            gauge!("sestats.total-messages", stats.total_messages as f64);
        }

        if EXPORT_CHATTERS {
            stats.chatters.into_iter().for_each(|chatter| {
                gauge!(
                    "sestats.chatter",
                    chatter.amount as f64,
                    &[("name", chatter.name.to_string()),]
                )
            });
        }

        if EXPORT_HASHTAGS {
            stats.hashtags.into_iter().for_each(|hashtag| {
                gauge!(
                    "sestats.hashtag",
                    hashtag.amount as f64,
                    &[("hashtag", hashtag.hashtag.to_string()),]
                )
            });
        }

        if EXPORT_COMMANDS {
            stats.commands.into_iter().for_each(|command| {
                gauge!(
                    "sestats.hashtag",
                    command.amount as f64,
                    &[("command", command.command.to_string()),]
                )
            });
        }

        if EXPORT_BTTV {
            stats.bttv_emotes.into_iter().for_each(|emote| {
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

        if EXPORT_FFZ {
            stats.ffz_emotes.into_iter().for_each(|emote| {
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

        if EXPORT_TWITCH {
            stats.twitch_emotes.into_iter().for_each(|emote| {
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

        if EXPORT_CHANNELS {
            top_channels.into_iter().for_each(|channel| {
                gauge!(
                    "sestats.channel",
                    channel.messages as f64,
                    &[("channel", channel.channel.to_string())]
                )
            });
        }

        debug!("Finished exporting stats")
    }
}
