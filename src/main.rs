mod asc;
mod bignumber;
mod chain;
mod common;
mod config;
mod database;
mod errors;
mod manifest_loader;
mod messages;
mod serializer;
mod subgraph;
mod subgraph_filter;
mod wasm_host;

use crate::subgraph_filter::SubgraphFilterInstance;
use config::Config;
use database::Database;
use errors::SwrError;
use manifest_loader::LoaderTrait;
use manifest_loader::ManifestLoader;
use messages::FilteredDataMessage;
use messages::SerializedDataMessage;
use messages::SourceDataMessage;
use serializer::Serializer;
use subgraph::Subgraph;
use wasm_host::create_wasm_host;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    // TODO: impl CLI
    let config = Config::load()?;

    // TODO: impl Source Consumer

    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.manifest).await?;

    // TODO: impl raw-data serializer
    let serializer = Serializer::new(config.clone())?;

    // TODO: impl subgraph filter
    let filter = SubgraphFilterInstance::new(&manifest)?;

    // TODO: impl Actual DB Connection
    let database = Database::new(&config).await?;

    let subgraph_id = config
        .subgraph_id
        .clone()
        .unwrap_or(config.subgraph_name.clone());

    let mut subgraph = Subgraph::new_empty(&config.subgraph_name, subgraph_id.to_owned());

    for datasource in manifest.datasources() {
        let api_version = datasource.mapping.apiVersion.to_owned();
        let wasm_bytes = manifest.load_wasm(&datasource.name).await?;
        let dbstore_agent = database.agent();
        let wasm_host = create_wasm_host(api_version, wasm_bytes, dbstore_agent)?;
        subgraph.create_source(wasm_host, datasource)?;
    }

    let (_sender1, recv1) = kanal::bounded_async::<SourceDataMessage>(1);
    let (sender2, recv2) = kanal::bounded_async::<SerializedDataMessage>(1);
    let (sender3, recv3) = kanal::bounded_async::<FilteredDataMessage>(1);

    let subscriber_run = async move { Ok::<(), SwrError>(()) };
    let serializer_run = serializer.run_async(recv1, sender2);
    let subgraph_filter_run = filter.run(recv2, sender3);
    let subgraph_run = subgraph.run_async(recv3);

    ::tokio::select! {
        result = subscriber_run => result,
        result = serializer_run => result.map_err(SwrError::from),
        result = subgraph_filter_run => result.map_err(SwrError::from),
        result = subgraph_run => result.map_err(SwrError::from),
        // TODO: impl prometheus
    }
}
