mod asc;
mod bignumber;
mod chain;
mod config;
mod core;
mod database;
mod errors;
mod from_to;
mod internal_messages;
mod manifest_loader;
mod subgraph;
mod wasm_host;

use config::Config;
use database::Database;
use errors::SwrError;
use kanal;
use manifest_loader::*;
use subgraph::Subgraph;
use subgraph::SubgraphSource;
use wasm_host::create_wasm_host;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    // 1. Load config & cli-arg
    // TODO: impl CLI
    let config = Config::load()?;
    // 2. TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.manifest).await?;
    // 3. Binding DB Connection
    let database = Database::new(&config).await?;
    // 4. Create a subgraph-instance
    let subgraph_id = config
        .subgraph_id
        .clone()
        .unwrap_or(config.subgraph_name.clone());
    let mut subgraph = Subgraph::new_empty(&config.subgraph_name, &subgraph_id);
    // Creating source & WasmHosts
    for datasource in manifest.datasources() {
        let api_version = datasource.mapping.apiVersion.to_owned();
        let wasm_bytes = manifest.load_wasm(&datasource.name).await?.wasm_bytes;
        let dbstore_agent = database.agent();
        let wasm_host = create_wasm_host(api_version, wasm_bytes, dbstore_agent)?;
        let subgraph_source = SubgraphSource::try_from((wasm_host, datasource))?;
        subgraph.add_source(subgraph_source);
    }

    // 5. Binding blockstore connection
    // TODO: impl blockstore (bus subscription)

    // 6. Creating message transport channel, moving one(1) mmessage at a time
    let (subgraph_msg_sender, subgraph_receiver) = kanal::bounded_async(1);
    // 7. Start 2 threads:
    // TODO: one thread for Input-Data(Block/Event/Log/Tx) Subscriber
    let subscriber_run = async move {
        ::log::info!("Acquire subgraph_sender: {:?}", subgraph_msg_sender);
        // todo!("Impl data subscription");
        Ok::<(), SwrError>(())
    };

    // - One thread for SubgraphWasmInstance
    let swr_run = subgraph.run_async(subgraph_receiver);

    // 8. Run until one of the threads stop
    ::tokio::select! {
        result = subscriber_run => result,
        result = swr_run => result.map_err(SwrError::from),
        // 9. TODO: We need prometheus as well
    }
}
