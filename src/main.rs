mod chain;
mod common;
mod components;
mod config;
mod errors;
mod logger_macros;
mod messages;
mod metrics;
mod runtime;

use components::database::DatabaseAgent;
use components::manifest_loader::LoaderTrait;
use components::manifest_loader::ManifestLoader;
use components::progress_ctrl::ProgressCtrl;
use components::rpc_client::RpcAgent;
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
use metrics::default_registry;
use metrics::run_metric_server;
use runtime::wasm_host::create_wasm_host;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    env_logger::try_init().unwrap_or_default();

    // TODO: impl CLI
    let config = Config::load()?;
    let registry = default_registry();

    let block_source = Source::new(&config).await?;
    // TODO: impl IPFS Loader
    let manifest = ManifestLoader::new(&config.subgraph_dir).await?;

    // TODO: impl raw-data serializer
    let serializer = Serializer::new(&config, registry)?;
    let filter = SubgraphFilter::new(config.chain.clone(), &manifest)?;
    let db = DatabaseAgent::new(&config, manifest.get_schema(), registry).await?;
    let rpc = RpcAgent::new(&config, manifest.get_abis().clone()).await?;
    let progress_ctrl =
        ProgressCtrl::new(db.clone(), manifest.get_sources(), config.reorg_threshold).await?;

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

    let results = tokio::join!(
        block_source.run_async(sender1),
        serializer.run_async(recv1, sender2),
        progress_ctrl.run_async(recv2, sender3),
        filter.run_async(recv3, sender4),
        subgraph.run_async(recv4, db, rpc),
        run_metric_server(config.metric_port.unwrap_or(8081))
    );

    log::info!("Results: {:?}", results);

    Ok(())
}
