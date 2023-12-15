mod datasource_wasm_instance;
mod metrics;

use super::ManifestAgent;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::BlockPtr;
use crate::common::DatasourceBundle;
use crate::common::HandlerTypes;
use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::debug;
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
use std::str::FromStr;
use std::time::Instant;
use web3::types::Address;

pub struct Subgraph {
    // NOTE: using IPFS might lead to subgraph-id using a hex/hash
    pub name: String,
    sources: Vec<(String, Option<String>, DatasourceWasmInstance)>, //name, address, instance
    metrics: SubgraphMetrics,
}

impl Subgraph {
    pub fn new_empty(config: &Config, registry: &Registry) -> Self {
        Self {
            sources: Vec::new(),
            name: config.subgraph_name.clone(),
            metrics: SubgraphMetrics::new(registry),
        }
    }

    fn create_single_source(
        &self,
        ds: &DatasourceBundle,
        manifest: &ManifestAgent,
        db: &DatabaseAgent,
        rpc: &RpcAgent,
        block_ptr: &BlockPtr,
    ) -> Result<DatasourceWasmInstance, SubgraphError> {
        let wasm_host = create_wasm_host(
            ds.api_version(),
            ds.wasm(),
            db.clone(),
            ds.name(),
            rpc.clone(),
            manifest.clone(),
            ds.address().into(),
            block_ptr.clone(),
            ds.network(),
        )
        .map_err(|e| SubgraphError::CreateSourceFail(e.to_string()))?;
        let source = DatasourceWasmInstance::try_from((wasm_host, ds.into()))?;
        Ok(source)
    }

    pub async fn create_sources(
        &mut self,
        manifest: &ManifestAgent,
        db: &DatabaseAgent,
        rpc: &RpcAgent,
        block_ptr: BlockPtr,
    ) -> Result<(), SubgraphError> {
        self.sources.clear();
        for datasource in manifest.datasources().iter() {
            let address = datasource.address();
            let name = datasource.name();
            let wasm_source =
                self.create_single_source(datasource, manifest, db, rpc, &block_ptr)?;
            self.sources.push((name, address, wasm_source));
        }
        Ok(())
    }

    async fn handle_ethereum_filtered_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
        manifest: &ManifestAgent,
        block_ptr: BlockPtr,
    ) -> Result<(), SubgraphError> {
        let mut block_handlers = HashMap::new();
        for (source_name, _, source_instance) in self.sources.iter_mut() {
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
            let (_, _, source_instance) = self
                .sources
                .iter_mut()
                .find(|(name, _, _)| name == &source_name)
                .ok_or(SubgraphError::InvalidSourceID(source_name.to_owned()))?;
            for handler in ethereum_handlers {
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }

        let timer = Instant::now();
        let event_count = events.len();
        for event in events {
            let count_datasources = self.sources.len();
            let source_instance = self
                .sources
                .iter_mut()
                .find(|(name, address, _)| {
                    if let Some(addr) = address {
                        let addr = Address::from_str(addr).unwrap();
                        name == &event.datasource && addr == event.event.address
                    } else {
                        name == &event.datasource
                    }
                })
                .map(|(_, _, source)| source);

            if source_instance.is_none() {
                continue;
            }

            let source_instance = source_instance.unwrap();

            source_instance.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;

            let count_new_datasouces = manifest.count_datasources() - count_datasources;
            if count_new_datasouces > 0 {
                let db = source_instance.host.db_agent.clone();
                let rpc = source_instance.host.rpc_agent.clone();
                for ds in manifest.datasources_take_last(count_new_datasouces) {
                    let wasm_source = self
                        .create_single_source(&ds, manifest, &db, &rpc, &block_ptr)
                        .unwrap();
                    self.sources.push((ds.name(), ds.address(), wasm_source));
                }
            }
        }
        if event_count > 0 {
            debug!(
                Subgraph,
                "processed all events in block";
                events => event_count,
                exec_time => format!("{:?}", timer.elapsed()),
                block => block_ptr.number
            );
        }

        Ok(())
    }

    async fn handle_filtered_data(
        &mut self,
        data: FilteredDataMessage,
        manifest: &ManifestAgent,
        block_ptr: BlockPtr,
    ) -> Result<(), SubgraphError> {
        match data {
            FilteredDataMessage::Ethereum { events, block } => {
                self.handle_ethereum_filtered_data(events, block, manifest, block_ptr)
                    .await
            }
        }
    }

    pub async fn process(
        &mut self,
        msg: FilteredDataMessage,
        manifest: &ManifestAgent,
    ) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        self.metrics
            .current_block_number
            .set(block_ptr.number as i64);

        self.handle_filtered_data(msg, manifest, block_ptr.clone())
            .await?;

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
