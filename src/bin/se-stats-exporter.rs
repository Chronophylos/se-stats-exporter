use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches};
use metrics::register_gauge;
use metrics_exporter_prometheus::PrometheusBuilder;
use se_stats_exporter::{export_stats, stats_api::ApiClient, ExportConfig, ExportName};
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::time;

fn get_matches() -> ArgMatches<'static> {
    App::new("se-stats-exporter")
        .arg(
            Arg::with_name("export")
                .long("export")
                .short("e")
                .help("Set what gets exported")
                .takes_value(true)
                .possible_values(&ExportName::variants())
                .use_delimiter(true)
                .default_value(
                    option_env!("SESTATS_EXPORT").unwrap_or("bttv,ffz,twitch,channel,chatter"),
                )
                .case_insensitive(true),
        )
        .arg(
            Arg::with_name("address")
                .long("address")
                .short("a")
                .help("Set the address for the prometheus scrape endpoint")
                .default_value(option_env!("SESTATS_ADDRESS").unwrap_or("127.0.0.1:9001")),
        )
        .arg(
            Arg::with_name("interval")
                .long("interval")
                .short("i")
                .help("Export interval in seconds")
                .long_help("How often the scape endpoint should get updated")
                .default_value(option_env!("SESTATS_INTERVAL").unwrap_or("10")),
        )
        .get_matches()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = get_matches();

    let export_config: ExportConfig =
        values_t_or_exit!(matches.values_of("export"), ExportName).into();
    let listen_addess = value_t_or_exit!(matches.value_of("address"), SocketAddr);
    let export_interval = value_t_or_exit!(matches.value_of("interval"), u64);

    PrometheusBuilder::new()
        .listen_address(listen_addess)
        .install()?;

    tracing_subscriber::fmt::init();

    register_gauge!("sestats.emote", "top emotes");
    register_gauge!("sestats.total-messages", "total messages on twitch");
    register_gauge!("sestats.chatter", "top chatters");
    register_gauge!("sestats.channel", "top channels");
    register_gauge!("sestats.command", "top commands");
    register_gauge!("sestats.hashtag", "top hashtags");

    let client = ApiClient::new()?;

    let mut interval = time::interval(Duration::from_secs(export_interval));

    loop {
        interval.tick().await;
        export_stats(&export_config, &client).await;
    }
}
