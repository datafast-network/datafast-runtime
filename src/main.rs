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
use std::fs;

fn welcome() {
    let contents =
        fs::read_to_string("./welcome.txt").expect("Should have been able to read the file");

    warn!(DatafastRuntime, "\nA product of Datafast - [df|runtime]");
    log::info!("\n\n{contents}");
}

fn handle_task_result<E: Debug>(r: Result<(), E>, task_name: &str) {
    info!(main, format!("{task_name} has finished"); result => format!("{:?}", r));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().unwrap_or_default();
    welcome();

    let config = Config::load();
    info!(main, "Config loaded");

    let registry = default_registry();

    // TODO: impl IPFS Loader
    let manifest = ManifestAgent::new(&config.subgraph_dir).await?;
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

    let filter = DataFilter::new(
        config.chain.clone(),
        manifest.datasource_and_templates(),
        manifest.get_abis(),
    )?;
    info!(main, "DataFilter ready");

    let rpc = RpcAgent::new(&config, manifest.get_abis()).await?;
    info!(main, "Rpc-Client ready");

    let mut subgraph = Subgraph::new_empty(&config, registry);
    info!(main, "Subgraph ready");

    let (sender, recv) = kanal::bounded_async(1);

    let query_blocks = block_source.run(sender, source_valve);

    let main_flow = async move {
        while let Ok(blocks) = recv.recv().await {
            info!(
                main,
                "block batch recevied and about to be processed";
                total_block => blocks.len()
            );

            valve.set_downloaded(&blocks);
            let time = std::time::Instant::now();
            let sorted_blocks = filter.filter_multi(blocks)?;
            let count_blocks = sorted_blocks.len();
            let last_block = sorted_blocks.last().map(|b| b.get_block_ptr());

            info!(
                main,
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

                if block_ptr.number % 10 == 0 {
                    subgraph
                        .create_sources(&manifest, &db, &rpc, block_ptr.clone())
                        .await?;
                }

                subgraph.process(block, &manifest).await?;

                rpc.clear_cache().await;

                if block_ptr.number % 500 == 0 {
                    valve.set_finished(block_ptr.number);
                }
            }

            if let Some(block_ptr) = last_block {
                db.commit_data(block_ptr.clone()).await?;
                db.remove_outdated_snapshots(block_ptr.number).await?;
                db.flush_cache().await?;

                if let Some(history_size) = config.block_data_retention {
                    if block_ptr.number > history_size {
                        db.clean_data_history(block_ptr.number - history_size)
                            .await?;
                    }
                }
            }

            info!(
                main,
                "block batch processed done";
                exec_time => format!("{:?}", time.elapsed()),
                number_of_blocks => count_blocks,
                avg_speed => format!("~{:?} blocks/sec", { count_blocks as u64 / time.elapsed().as_secs() })
            );
        }

        warn!(main, "No more messages returned from block-stream");
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tokio::select!(
        r = query_blocks => handle_task_result(r, "block-source"),
        r = main_flow => handle_task_result(r, "Main flow stopped"),
        _ = run_metric_server(config.metric_port.unwrap_or(8081)) => ()
    );

    Ok(())
}
