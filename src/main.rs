mod chain;
mod common;
mod components;
mod config;
mod errors;
mod logger_macros;
mod messages;
mod runtime;

use components::database::Database;
use components::manifest_loader::LoaderTrait;
use components::manifest_loader::ManifestLoader;
use components::serializer::Serializer;
use components::source::Source;
use components::subgraph::Subgraph;
use components::subgraph_filter::SubgraphFilter;
use components::subgraph_filter::SubgraphFilterTrait;
use config::Config;
use errors::SwrError;
use messages::FilteredDataMessage;
use messages::SerializedDataMessage;
use messages::SourceDataMessage;
use runtime::wasm_host::create_wasm_host;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    env_logger::try_init().unwrap_or_default();
    // TODO: impl CLI
    let config = Config::load()?;
    // TODO: impl Source Consumer with Nats
    let block_source = Source::new(&config).await?;

    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.subgraph_dir).await?;

    // TODO: impl raw-data serializer
    let serializer = Serializer::new(config.clone())?;

    // TODO: impl subgraph filter
    let subgraph_filter = SubgraphFilter::new(config.chain.clone(), &manifest)?;

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
        let wasm_host = create_wasm_host(
            api_version,
            wasm_bytes,
            dbstore_agent,
            datasource.name.clone(),
        )?;
        subgraph.create_source(wasm_host, datasource)?;
    }

    let (sender1, recv1) = kanal::bounded_async::<SourceDataMessage>(1);
    let (sender2, recv2) = kanal::bounded_async::<SerializedDataMessage>(1);
    let (sender3, recv3) = kanal::bounded_async::<FilteredDataMessage>(1);

    let stream_run = block_source.run_async(sender1);
    let serializer_run = serializer.run_async(recv1, sender2);
    let subgraph_filter_run = subgraph_filter.run_async(recv2, sender3);
    let subgraph_run = subgraph.run_async(recv3);

    let results = ::tokio::join!(
        stream_run,
        serializer_run,
        subgraph_filter_run,
        subgraph_run,
    );
    log::info!("Results: {:?}", results);
    Ok(())
}
