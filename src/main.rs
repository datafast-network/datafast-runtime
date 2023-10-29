mod asc;
mod bignumber;
mod chain;
mod config;
mod core;
mod errors;
mod host_exports;
mod manifest_loader;
mod subgraph;

use config::Config;
use errors::SwrError;
use host_exports::create_wasm_host_instance;
use kanal;
use manifest_loader::ManifestLoader;
use std::collections::HashMap;
use std::sync::Arc;
use subgraph::Handler;
use subgraph::Subgraph;
use subgraph::SubgraphOperationMessage;
use subgraph::SubgraphSource;

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    // 1. Load config & cli-arg
    let config = Config::load()?;

    // 2. Start ManifestLoader & load data
    let manifest = ManifestLoader::new(&config).await?;

    // 3. Binding db connection
    // TODO: add DB binding connection, so we can impl store_set & store_get

    // 4. Create a subgraph-instance first
    // NOTE: normally subgraph does not have a name. It generally is derived from the hash of the whole manifest set.
    // But for now, for the sake of simplicity, pass subgraph-id to Config
    let mut subgraph = Subgraph::new_empty(&config.subgraph_id);

    for source in manifest.datasources {
        // Creating host instance
        let wasm_bytes = manifest.load_wasm(source.name).await?;
        let mut host = create_wasm_host_instance(source.version, wasm_bytes)?;

        let mut handlers = HashMap::new();

        for (event_name, handler_name) in source.event_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(event_name.to_owned(), handler);
        }

        // FIXME: Saving the 2 following set of handlers might lead to Naming-collision
        for handler_name in source.block_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(handler_name.to_owned(), handler);
        }

        for (_, handler_name) in source.tx_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(handler_name.to_owned(), handler);
        }

        let subgraph_source = SubgraphSource {
            id: source.name.to_owned(),
            handlers,
            host,
        };

        subgraph.add_source(subgraph_source);
    }

    // 5. Binding blockstore connection

    // 6. Creating message transport channel
    let (sender, receiver) = kanal::bounded::<SubgraphOperationMessage>(1);
    let sender = Arc::new(sender);

    Ok(())
}
