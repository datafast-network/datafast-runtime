mod asc;
mod bignumber;
mod chain;
mod common;
mod config;
mod database;
mod errors;
mod from_to;
mod manifest_loader;
mod messages;
mod subgraph;
mod wasm_host;

mod subgraph_filter;

use config::Config;
use database::Database;
use errors::SwrError;
use manifest_loader::*;
use subgraph::DatasourceWasmInstance;
use subgraph::Subgraph;
use wasm_host::create_wasm_host;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    // TODO: impl CLI
    let config = Config::load()?;
    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.manifest).await?;
    // TODO: impl Actual DB Connection
    let database = Database::new(&config).await?;

    let subgraph_id = config
        .subgraph_id
        .clone()
        .unwrap_or(config.subgraph_name.clone());

    let mut subgraph = Subgraph::new_empty(&config.subgraph_name, &subgraph_id);

    for datasource in manifest.datasources() {
        let api_version = datasource.mapping.apiVersion.to_owned();
        let wasm_bytes = manifest.load_wasm(&datasource.name).await?;
        let dbstore_agent = database.agent();
        let wasm_host = create_wasm_host(api_version, wasm_bytes, dbstore_agent)?;
        let subgraph_source = DatasourceWasmInstance::try_from((wasm_host, datasource))?;
        subgraph.add_source(subgraph_source);
    }
    // TODO: impl transform data instance
    let (_transform_sender, data_input_receiver) = kanal::bounded_async(1);
    // TODO: impl blockstore (bus subscription)

    let (subgraph_msg_sender, subgraph_receiver) = kanal::bounded_async(1);
    let subgraph_filter = subgraph_filter::SubgraphFilterInstance::new(
        &manifest,
        subgraph_msg_sender,
        data_input_receiver,
    )?;
    // TODO: pass block-store subscriber to thread
    let subscriber_run = async move { Ok::<(), SwrError>(()) };
    let swr_run = subgraph.run_async(subgraph_receiver);
    let filter_run = subgraph_filter.run();

    ::tokio::select! {
        result = subscriber_run => result,
        result = swr_run => result.map_err(SwrError::from),
        result = filter_run => result.map_err(SwrError::from),
        // TODO: impl prometheus
    }
}
