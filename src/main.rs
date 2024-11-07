mod chain;
mod common;
mod components;
mod config;
mod database;
mod errors;
mod metrics;
mod processor;
mod proto;
mod rpc_client;
mod runtime;

use components::*;
use config::Config;
use df_logger::critical;
use df_logger::debug;
use df_logger::error;
use df_logger::info;
use df_logger::loggers::init_logger;
use df_logger::warn;
use metrics::default_registry;
use metrics::run_metric_server;
use processor::Processor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();

    let config = Config::load();
    info!(main, "Config loaded!");
    let registry = default_registry();
    let processor = Processor::default();

    tokio::select!(
        r = processor.run(&config, registry) => {
            if let Err(e) = r {
                critical!(main, "Processor failed!"; error => format!("{:?}", e));
                panic!("{:?}", e);
            }
        },
        _ = tokio::spawn(run_metric_server(config.metric_port.unwrap_or(8081))) => ()
    );

    Ok(())
}
