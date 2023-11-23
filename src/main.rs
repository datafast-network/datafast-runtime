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
use messages::FilteredDataMessage;
use messages::SerializedDataMessage;
use messages::SourceDataMessage;
use metrics::default_registry;
use metrics::run_metric_server;
use rpc_client::RpcAgent;
use runtime::wasm_host::create_wasm_host;
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

    let manifest = ManifestLoader::new(&config.subgraph_dir).await?;
    let db = DatabaseAgent::new(&config, manifest.get_schema(), registry).await?;
    let progress_ctrl =
        ProgressCtrl::new(db.clone(), manifest.get_sources(), config.reorg_threshold).await?;

    let block_source = Source::new(&config, progress_ctrl.clone()).await?;
    // TODO: impl IPFS Loader

    // TODO: impl raw-data serializer
    let serializer = Serializer::new(&config, registry)?;
    let filter = SubgraphFilter::new(config.chain.clone(), &manifest)?;
    let rpc = RpcAgent::new(&config, manifest.get_abis().clone()).await?;

    let mut subgraph = Subgraph::new_empty(&config, registry);

    for datasource in manifest.datasources() {
        let api_version = datasource.mapping.apiVersion.to_owned();
        let wasm_bytes = manifest.load_wasm(&datasource.name).await?;
        let wasm_host = create_wasm_host(
            api_version,
            wasm_bytes,
            db.clone(),
            datasource.name.clone(),
            rpc.clone(),
        )?;
        subgraph.create_source(wasm_host, datasource)?;
    }

    let (sender1, recv1) = kanal::bounded_async::<SourceDataMessage>(1);
    let (sender2, recv2) = kanal::bounded_async::<SerializedDataMessage>(1);
    let (sender3, recv3) = kanal::bounded_async::<SerializedDataMessage>(1);
    let (sender4, recv4) = kanal::bounded_async::<FilteredDataMessage>(1);

    tokio::select!(
        r = block_source.run_async(sender1, sender2.clone()) => handle_task_result(r, "block-source"),
        r = serializer.run_async(recv1, sender2) => handle_task_result(r, "Serializer"),
        r = progress_ctrl.run_async(recv2, sender3) => handle_task_result(r, "ProgressCtrl"),
        r = filter.run_async(recv3, sender4) => handle_task_result(r, "SubgraphFilter"),
        r = subgraph.run_async(recv4, db, rpc) => handle_task_result(r, "Subgraph"),
        _ = run_metric_server(config.metric_port.unwrap_or(8081)) => ()
    );

    Ok(())
}
