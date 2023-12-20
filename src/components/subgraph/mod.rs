mod datasource_wasm_instance;
mod metrics;

use super::ManifestAgent;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::EthereumFilteredEvent;
use crate::common::FilteredDataMessage;
use crate::common::HandlerTypes;
use crate::database::DatabaseAgent;
use crate::errors::SubgraphError;
use crate::info;
use crate::rpc_client::RpcAgent;
use datasource_wasm_instance::DatasourceWasmInstance;
use metrics::SubgraphMetrics;
use prometheus::Registry;
use std::collections::HashMap;
use std::time::Instant;

pub struct Subgraph {
    sources: HashMap<(String, Option<String>), DatasourceWasmInstance>,
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
            sources: HashMap::new(),
            metrics: SubgraphMetrics::new(registry),
            rpc: rpc.clone(),
            db: db.clone(),
            manifest: manifest.clone(),
        }
    }

    pub fn should_process(&self, data: &FilteredDataMessage) -> bool {
        match data {
            FilteredDataMessage::Ethereum { events, .. } => {
                return events.len() > 0
                    || self
                        .sources
                        .values()
                        .find(|ds| !ds.ethereum_handlers.block.is_empty())
                        .is_some();
            }
        }
    }

    pub fn create_sources_if_needed(&mut self) -> Result<(), SubgraphError> {
        let timer = self.metrics.datasource_creation_duration.start_timer();
        if !self.sources.is_empty() {
            for current_source in self.sources.values_mut() {
                if current_source.should_reset() {
                    self.sources.clear();
                    break;
                }
            }
        }

        if self.sources.is_empty() {
            for ds in self.manifest.datasource_and_templates().inner() {
                self.sources.insert(
                    (ds.name(), ds.address()),
                    DatasourceWasmInstance::try_from((
                        ds,
                        self.db.clone(),
                        self.rpc.clone(),
                        self.manifest.clone(),
                    ))?,
                );
            }
            self.metrics.datasource_creation_counter.inc();
        }

        timer.stop_and_record();
        Ok(())
    }

    fn handle_ethereum_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        let mut block_handlers = HashMap::new();

        for ((source_name, _), source_instance) in self.sources.iter() {
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
            let (_, source_instance) = self
                .sources
                .iter_mut()
                .find(|((name, _), _)| name == &source_name)
                .ok_or(SubgraphError::InvalidSourceID(source_name.to_owned()))?;
            for handler in ethereum_handlers {
                self.metrics.eth_trigger_counter.inc();
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }

        for event in events {
            let ds_name = event.datasource.clone();
            let handler_name = event.handler.clone();
            let event_address = format!("{:?}", event.event.address).to_lowercase();

            if let Some(source) = self
                .sources
                .get_mut(&(ds_name.clone(), Some(event_address)))
            {
                self.metrics.eth_trigger_counter.inc();
                let timer = self
                    .metrics
                    .eth_event_process_duration
                    .with_label_values(&[&ds_name, &handler_name])
                    .start_timer();
                source.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;
                timer.stop_and_record();
                continue;
            }

            if let Some(source) = self.sources.get_mut(&(ds_name.clone(), None)) {
                if !self.manifest.should_process_address(
                    &event.datasource,
                    &format!("{:?}", event.event.address).to_lowercase(),
                ) {
                    // NOTE: This datasource is based from a template,
                    // and this address is not relevant to process
                    continue;
                }

                // NOTE: This datasource is either based from a template or a no-address datasource,
                // this address might be relevant if the datasource is template, or
                // directly relevant to the no-address datasource
                self.metrics.eth_trigger_counter.inc();
                let timer = self
                    .metrics
                    .eth_event_process_duration
                    .with_label_values(&[&ds_name, &handler_name])
                    .start_timer();
                source.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;
                timer.stop_and_record();
            }

            continue;
        }

        Ok(())
    }

    pub fn process(&mut self, msg: FilteredDataMessage) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        self.metrics
            .current_block_number
            .set(block_ptr.number as i64);

        let timer = self.metrics.block_process_duration.start_timer();
        match msg {
            FilteredDataMessage::Ethereum { events, block } => {
                self.handle_ethereum_data(events, block)?
            }
        };
        timer.stop_and_record();

        if block_ptr.number % 1000 == 0 {
            info!(
                Subgraph,
                format!("finished processing block #{}", block_ptr.number)
            );
        }

        Ok(())
    }
}
