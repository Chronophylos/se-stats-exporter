use se_stats_exporter::{export_stats, stats_api::ApiClient, ExportConfig};

#[tokio::main]
async fn main() {
    let client = ApiClient::new().unwrap();
    let config = ExportConfig::all();

    export_stats(&config, &client).await;
}
