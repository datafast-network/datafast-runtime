mod datasource_wasm_instance;
mod metrics;

use super::ManifestLoader;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::HandlerTypes;
use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::errors::SubgraphError;
use crate::info;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::rpc_client::RpcAgent;
use crate::runtime::wasm_host::create_wasm_host;
use datasource_wasm_instance::DatasourceWasmInstance;
use metrics::SubgraphMetrics;
use prometheus::Registry;
use std::collections::HashMap;

pub struct Subgraph {
    // NOTE: using IPFS might lead to subgraph-id using a hex/hash
    pub name: String,
    sources: HashMap<String, DatasourceWasmInstance>,
    metrics: SubgraphMetrics,
}

impl Subgraph {
    pub fn new_empty(config: &Config, registry: &Registry) -> Self {
        Self {
            sources: HashMap::new(),
            name: config.subgraph_name.clone(),
            metrics: SubgraphMetrics::new(registry),
        }
    }

    pub async fn create_sources(
        &mut self,
        manifest: &ManifestLoader,
        db: &DatabaseAgent,
        rpc: &RpcAgent,
    ) -> Result<(), SubgraphError> {
        self.sources.clear();
        for datasource in manifest.datasources() {
            let api_version = datasource.mapping.apiVersion.to_owned();
            let wasm_bytes = manifest
                .load_wasm(&datasource.name)
                .await
                .map_err(|e| SubgraphError::CreateSourceFail(e.to_string()))?;
            let wasm_host = create_wasm_host(
                api_version,
                wasm_bytes,
                db.clone(),
                datasource.name.clone(),
                rpc.clone(),
            )
            .map_err(|e| SubgraphError::CreateSourceFail(e.to_string()))?;
            let source = DatasourceWasmInstance::try_from((wasm_host, datasource))?;
            self.sources.insert(source.id.clone(), source);
        }
        Ok(())
    }

    fn handle_ethereum_filtered_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        let mut block_handlers = HashMap::new();
        for (source_name, source_instance) in self.sources.iter_mut() {
            let source_block_handlers = source_instance
                .ethereum_handlers
                .block
                .keys()
                .cloned()
                .collect::<Vec<String>>();
            block_handlers.insert(source_name.to_owned(), source_block_handlers);
        }

        for (source_name, ethereum_handlers) in block_handlers {
            // FIXME: this is not correct, block-handler may have filter itself,
            // thus not all datasource would handle the same block
            let source_instance = self.sources.get_mut(&source_name).unwrap();
            for handler in ethereum_handlers {
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }

        for event in events {
            let source_instance = self
                .sources
                .get_mut(&event.datasource)
                .ok_or(SubgraphError::InvalidSourceID(event.datasource.to_owned()))?;
            source_instance.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;
        }

        Ok(())
    }

    fn handle_filtered_data(&mut self, data: FilteredDataMessage) -> Result<(), SubgraphError> {
        match data {
            FilteredDataMessage::Ethereum { events, block } => {
                self.handle_ethereum_filtered_data(events, block)
            }
        }
    }

    pub async fn process(&mut self, msg: FilteredDataMessage) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        self.metrics
            .current_block_number
            .set(block_ptr.number as i64);

        let timer = self.metrics.block_process_duration.start_timer();
        self.handle_filtered_data(msg)?;
        timer.stop_and_record();
        self.metrics.block_process_counter.inc();

        if block_ptr.number % 1000 == 0 {
            info!(
                Subgraph,
                "finished processing block";
                block_number => block_ptr.number,
                block_hash => block_ptr.hash
            );
        }

        Ok(())
    }
}
