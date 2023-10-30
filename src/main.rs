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
    // when using ManifestLoader to load bundle from IPFS, we might actually grab subgraph-hex-id
    // but since it is not implemented yet, use config's subgraph_name for id as well
    // This value is not critical, basically it is meant to help with metrics (prometheus) later
    let subgraph_id = config
        .subgraph_id
        .clone()
        .unwrap_or(config.subgraph_name.clone());
    let mut subgraph = Subgraph::new_empty(&config.subgraph_name, &subgraph_id);

    for source in manifest.datasources.iter() {
        let wasm_bytes = manifest.load_wasm(&source.name).await?;
        let wasm_host = create_wasm_host_instance(source.version.to_owned(), wasm_bytes)?;
        let subgraph_source = SubgraphSource::try_from((wasm_host, source.to_owned()))?;
        subgraph.add_source(subgraph_source);
    }

    // 5. Binding blockstore connection
    // TODO: impl blockstore (bus subscription)

    // 6. Creating message transport channel
    // Receving one mmessage at a time
    let (subgraph_msg_sender, subgraph_receiver) =
        kanal::bounded_async::<SubgraphOperationMessage>(1);
    let (store_sender, store_receiver) = kanal::bounded_async(1);

    // 7. Start 3 threads:
    // - One thread for Input-Data(Block/Event/Log/Tx) Subscriber
    // TODO: impl
    let subscriber_run = async move {
        ::log::info!("Acquire subgraph_sender: {:?}", subgraph_msg_sender);
        // todo!("Impl data subscription");
        Ok::<(), SwrError>(())
    };

    // - One thread for SubgraphWasmInstance
    let swr_run = subgraph.run_async(subgraph_receiver, store_sender);

    // - One thread for DatabaseWorker
    // TODO: impl
    let database_worker_run = async move {
        ::log::info!("Acquire store_receiver: {:?}", store_receiver);
        // todo!("Impl database worker");
        Ok::<(), SwrError>(())
    };

    // 8. Run until one of the threads stop
    ::tokio::select! {
        result = subscriber_run => {
            match result {
                Ok(()) => {
                    ::log::info!("InputSubscriber stopped successfully");
                    Ok(())
                },
                Err(e) => {
                    ::log::error!("InputSubscriber stopped unexpectedly: {:?}", e);
                    return Err(SwrError::from(e));
                },
            }
        }
        result = swr_run => {
            match result {
                Ok(()) => {
                    ::log::info!("SubgraphWasmHost stopped successfully");
                    Ok(())
                },
                Err(e) => {
                    ::log::error!("SubgraphWasmHost stopped unexpectedly: {:?}", e);
                    return Err(SwrError::from(e));
                },
            }
        }
        result = database_worker_run => {
            match result {
                Ok(()) => {
                    ::log::info!("DatabaseWorker stopped successfully");
                    Ok(())
                },
                Err(e) => {
                    ::log::error!("DatabaseWorker stopped unexpectedly: {:?}", e);
                    return Err(SwrError::from(e));
                },
            }
        }
    }
}
