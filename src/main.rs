#![warn(missing_copy_implementations, missing_debug_implementations)]

use anyhow::Result;
use metrics::{counter, gauge, register_counter, register_gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::time;
use tracing::error;

mod stats_api;
// mod stats_ws;

#[tokio::main]
async fn main() -> Result<()> {
    PrometheusBuilder::new()
        .listen_address(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            9001,
        ))
        .install()?;

    register_gauge!("sestats.emote", "number of used emotes",);

    let client = stats_api::ApiClient::new()?;

    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;

        let stats = match client.get_stats("global").await {
            Err(e) => {
                error!("Could not get stats from stats.streamelements.com");
                continue;
            }
            Ok(s) => s,
        };
        let bttv_emotes = stats.bttv_emotes.to_vec();

        debug!("Exporting stats to Prometheus");

        for emote in bttv_emotes.iter() {
            gauge!(
                "sestats.emote",
                emote.amount as f64,
                &[
                    ("provider", String::from("bttv")),
                    ("emote", emote.emote.to_string()),
                ]
            );
        }
    }
}
