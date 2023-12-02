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
use messages::SerializedDataMessage;
use metrics::default_registry;
use metrics::run_metric_server;
use rpc_client::RpcAgent;
use std::fmt::Debug;

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
    let valve = Valve::new(&config.valve);
    let db = DatabaseAgent::new(&config, manifest.get_schema(), registry).await?;
    let mut progress_ctrl =
        ProgressCtrl::new(db.clone(), manifest.get_sources(), config.reorg_threshold).await?;
    let block_source = BlockSource::new(&config, progress_ctrl.clone()).await?;
    let filter = SubgraphFilter::new(config.chain.clone(), &manifest)?;
    let rpc = RpcAgent::new(&config, manifest.get_abis().clone()).await?;
    let mut subgraph = Subgraph::new_empty(&config, registry);
    let source_valve = valve.clone();

    let (sender, recv) = kanal::bounded_async::<Vec<SerializedDataMessage>>(1);

    let main_flow = async move {
        while let Ok(blocks) = recv.recv().await {
            info!(
                MainFlow,
                "block batch recevied and about to be processed";
                total_block => blocks.len()
            );

            let time = std::time::Instant::now();
            let blocks = filter.filter_multi(blocks)?;
            let count_blocks = blocks.len();

            info!(
                MainFlow,
                "filter processed OK";
                exec_time => format!("{:?}", time.elapsed()),
                count_blocks => count_blocks
            );

            let time = std::time::Instant::now();

            for block in blocks {
                subgraph.create_sources(&manifest, &db, &rpc).await?;
                progress_ctrl.check_block(block.get_block_ptr()).await?;
                subgraph.process(block, &db, &rpc, &valve).await?;
            }

            info!(
                MainFlow,
                "block batch processed OK";
                exec_time => format!("{:?}", time.elapsed()),
                count => count_blocks
            );
        }

        warn!(MainFlow, "No more messages returned from block-stream");
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tokio::select!(
        r = block_source.run_async(sender, source_valve) => handle_task_result(r, "block-source"),
        r = main_flow => handle_task_result(r, "Main flow stopped"),
        _ = run_metric_server(config.metric_port.unwrap_or(8081)) => ()
    );

    Ok(())
}
