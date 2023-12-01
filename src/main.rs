mod chain;
mod common;
mod components;
mod config;
mod database;
mod errors;
mod logger_macros;
mod messages;
mod metrics;
mod rpc_client;
mod runtime;
mod schema_lookup;

use components::*;
use config::Config;
use database::DatabaseAgent;
use metrics::default_registry;
use metrics::run_metric_server;
use rpc_client::RpcAgent;
use std::fmt::Debug;
use tokio::spawn;

fn handle_task_result<E: Debug>(r: Result<(), E>, task_name: &str) {
    info!(main, format!("{task_name} has finished"); result => format!("{:?}", r));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().unwrap_or_default();

    // TODO: impl CLI
    let config = Config::load();
    let registry = default_registry();

    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.subgraph_dir).await?;
    let db = DatabaseAgent::new(&config, manifest.get_schema(), registry).await?;
    let mut progress_ctrl =
        ProgressCtrl::new(db.clone(), manifest.get_sources(), config.reorg_threshold).await?;

    let block_source = BlockSource::new(&config, progress_ctrl.clone()).await?;

    let filter = SubgraphFilter::new(config.chain.clone(), &manifest)?;
    let rpc = RpcAgent::new(&config, manifest.get_abis().clone()).await?;

    let mut subgraph =
        Subgraph::new_empty(&config, registry, db.clone(), rpc.clone(), manifest).await;

    let (sender2, recv2) = kanal::bounded_async(1);

    tokio::select!(
        r = spawn(block_source.run_async(sender2)) => handle_task_result(r.unwrap(), "block-source"),
        r = async move {

            while let Ok(messages) = recv2.recv().await {
                info!(main, "message batch recevied and about to be processed");
                for msg in messages {
                    progress_ctrl.run_sync(msg.get_block_ptr()).await?;
                    let ok_msg = filter.run_sync(msg).await?;
                    subgraph.run_sync(ok_msg, &db, &rpc).await?;
                }
            };

            warn!(MainFlow, "No more messages returned from block-stream");
            Ok::<(), Box<dyn std::error::Error>>(())
        } => handle_task_result(r, "Main flow stopped"),
        _ = run_metric_server(config.metric_port.unwrap_or(8081)) => ()
    );

    Ok(())
}
