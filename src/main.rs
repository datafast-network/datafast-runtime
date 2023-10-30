mod asc;
mod bignumber;
mod chain;
mod config;
mod core;
mod errors;
mod from_to;
mod host_exports;
mod manifest_loader;
mod subgraph;

use config::Config;
use errors::SwrError;
use host_exports::create_wasm_host_instance;
use kanal;
use manifest_loader::ManifestLoader;
use std::sync::Arc;
use subgraph::Subgraph;
use subgraph::SubgraphOperationMessage;
use subgraph::SubgraphSource;

/*
The goal design is, the runtime must be very easy to use, very easy to pull a demo
Example usage:
$ swr --manifest ~/my-subgraph-repo --subscribe nats://localhost:9000/blocks --store mystore://localhost:12345/namespace
*/

#[tokio::main]
async fn main() -> Result<(), SwrError> {
    // 1. Load config & cli-arg
    let config = Config::load()?;

    // 2. Start ManifestLoader & load manifest bundle (subgraph.yaml/abis/wasm etc)
    let manifest = ManifestLoader::new(&config).await?;

    // 3. Binding db connection
    // TODO: add DB binding connection, so we can impl store_set & store_get

    // 4. Create a subgraph-instance first
    // NOTE: normally subgraph does not have a name. It generally is derived from the hash of the whole manifest set.
    // But for now, for the sake of simplicity, pass subgraph-id to Config
    let mut subgraph = Subgraph::new_empty(&config.subgraph_id);

    for source in manifest.datasources.iter() {
        // Creating host instance
        let wasm_bytes = manifest.load_wasm(&source.name).await?;
        let host = create_wasm_host_instance(source.version.to_owned(), wasm_bytes)?;
        let subgraph_source = SubgraphSource::try_from((host, source.to_owned()))?;
        subgraph.add_source(subgraph_source);
    }

    // 5. Binding blockstore connection

    // 6. Creating message transport channel
    // Receving one mmessage at a time
    let (sender, receiver) = kanal::bounded::<SubgraphOperationMessage>(1);
    let sender = Arc::new(sender);

    // 7. Start threads for subgraph-source and invoke

    Ok(())
}
