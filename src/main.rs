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
use messages::BlockDataMessage;
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

    let config = Config::load();
    info!(main, "Config loaded");

    let registry = default_registry();

    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.subgraph_dir).await?;
    info!(main, "Manifest loaded");

    let valve = Valve::new(&config.valve);
    let source_valve = valve.clone();

    let db = DatabaseAgent::new(&config.database, manifest.get_schema(), registry).await?;
    info!(main, "Database set up");

    let mut inspector = Inspector::new(
        db.get_recent_block_pointers(config.reorg_threshold).await?,
        manifest.get_sources(),
        config.reorg_threshold,
    );
    info!(main, "BlockInspector ready"; next_start_block => inspector.get_expected_block_number());

    let block_source = BlockSource::new(&config, inspector.get_expected_block_number()).await?;
    info!(main, "BlockSource ready");

    let filter = DataFilter::new(config.chain.clone(), manifest.datasources())?;
    info!(main, "DataFilter ready");

    let rpc = RpcAgent::new(&config, manifest.get_abis().clone()).await?;
    info!(main, "Rpc-Client ready");

    let mut subgraph = Subgraph::new_empty(&config, registry);
    info!(main, "Subgraph ready");

    let (sender, recv) = kanal::bounded_async::<Vec<BlockDataMessage>>(1);

    let main_flow = async move {
        while let Ok(blocks) = recv.recv().await {
            info!(
                MainFlow,
                "block batch recevied and about to be processed";
                total_block => blocks.len()
            );

            valve.set_downloaded(&blocks);
            let time = std::time::Instant::now();
            let sorted_blocks = filter.filter_multi(blocks)?;
            let count_blocks = sorted_blocks.len();
            let last_block = sorted_blocks.last().map(|b| b.get_block_ptr());

            info!(
                MainFlow,
                "block data got filtered";
                exec_time => format!("{:?}", time.elapsed()),
                count_blocks => count_blocks
            );

            let time = std::time::Instant::now();

            for block in sorted_blocks {
                let block_ptr = block.get_block_ptr();
                rpc.set_block_ptr(&block_ptr).await;

                match inspector.check_block(block_ptr.clone()) {
                    BlockInspectionResult::UnexpectedBlock
                    | BlockInspectionResult::UnrecognizedBlock => {
                        panic!("Bad block data from source");
                    }
                    BlockInspectionResult::BlockAlreadyProcessed
                    | BlockInspectionResult::MaybeReorg => {
                        continue;
                    }
                    BlockInspectionResult::ForkBlock => {
                        db.revert_from_block(block_ptr.number).await?;
                    }
                    BlockInspectionResult::OkToProceed => (),
                };

                subgraph.create_sources(&manifest, &db, &rpc).await?;
                subgraph.process(block).await?;
                valve.set_finished(block_ptr.number);
            }

            if let Some(block_ptr) = last_block {
                db.commit_data(block_ptr.clone()).await?;
                db.flush_cache().await?;
            }

            info!(
                MainFlow,
                "block batch processed done";
                exec_time => format!("{:?}", time.elapsed()),
                number_of_blocks => count_blocks,
                avg_speed => format!("~{:?} blocks/sec", { count_blocks as u64 / time.elapsed().as_secs() })
            );
        }

        warn!(MainFlow, "No more messages returned from block-stream");
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tokio::select!(
        r = block_source.run(sender, source_valve) => handle_task_result(r, "block-source"),
        r = main_flow => handle_task_result(r, "Main flow stopped"),
        _ = run_metric_server(config.metric_port.unwrap_or(8081)) => ()
    );

    Ok(())
}
