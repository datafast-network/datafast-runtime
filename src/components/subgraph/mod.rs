mod datasource_wasm_instance;
mod metrics;

use super::ManifestAgent;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::HandlerTypes;
use crate::database::DatabaseAgent;
use crate::debug;
use crate::errors::SubgraphError;
use crate::info;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::rpc_client::RpcAgent;
use datasource_wasm_instance::DatasourceWasmInstance;
use metrics::SubgraphMetrics;
use prometheus::Registry;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use web3::types::Address;

pub struct Subgraph {
    sources: Vec<(String, Option<String>, DatasourceWasmInstance)>,
    metrics: SubgraphMetrics,
    rpc: RpcAgent,
    db: DatabaseAgent,
    manifest: ManifestAgent,
}

impl Subgraph {
    pub fn new(
        db: &DatabaseAgent,
        rpc: &RpcAgent,
        manifest: &ManifestAgent,
        registry: &Registry,
    ) -> Self {
        Self {
            sources: Vec::new(),
            metrics: SubgraphMetrics::new(registry),
            rpc: rpc.clone(),
            db: db.clone(),
            manifest: manifest.clone(),
        }
    }

    pub fn create_sources(&mut self) -> Result<(), SubgraphError> {
        self.sources.clear();
        for ds in self.manifest.datasources().iter() {
            self.sources.push((
                ds.name(),
                ds.address(),
                DatasourceWasmInstance::try_from((
                    ds,
                    self.db.clone(),
                    self.rpc.clone(),
                    self.manifest.clone(),
                ))?,
            ));
        }
        Ok(())
    }

    fn check_for_new_datasource(&mut self) -> Result<usize, SubgraphError> {
        let active_ds_instance_count = self.sources.len();
        let pending_ds = self.manifest.count_datasources() - active_ds_instance_count;

        if pending_ds == 0 {
            return Ok(0);
        }

        for ds in self.manifest.datasources_take_last(pending_ds) {
            self.sources.push((
                ds.name(),
                ds.address(),
                DatasourceWasmInstance::try_from((
                    &ds,
                    self.db.clone(),
                    self.rpc.clone(),
                    self.manifest.clone(),
                ))?,
            ));
        }

        Ok(pending_ds)
    }

    fn handle_ethereum_filtered_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        let block_number = block.number.as_u64();
        let mut block_handlers = HashMap::new();

        for (source_name, _, source_instance) in self.sources.iter() {
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
            self.check_for_new_datasource()?;
        }

        if event_count > 0 {
            debug!(
                Subgraph,
                "processed all events in block";
                events => event_count,
                exec_time => format!("{:?}", timer.elapsed()),
                block => block_number
            );
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

    pub fn process(&mut self, msg: FilteredDataMessage) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        self.metrics
            .current_block_number
            .set(block_ptr.number as i64);

        self.handle_filtered_data(msg)?;

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
