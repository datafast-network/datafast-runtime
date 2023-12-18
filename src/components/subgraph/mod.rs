mod datasource_wasm_instance;
mod metrics;

use super::ManifestAgent;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::EthereumFilteredEvent;
use crate::common::FilteredDataMessage;
use crate::common::HandlerTypes;
use crate::database::DatabaseAgent;
use crate::debug;
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
    create_source_count: u64,
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
            create_source_count: 0,
        }
    }

    pub fn create_sources(&mut self, block: u64) -> Result<(), SubgraphError> {
        let time = Instant::now();
        if self.sources.is_empty() {
            self.create_source_count += 1;
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
            info!(
                Subgraph, "created wasm-datasources 💥";
                count => self.create_source_count,
                block => block,
                exec_time => format!("{:?}", time.elapsed())
            );
        } else {
            for current_source in self.sources.values_mut() {
                if current_source.should_reset() {
                    self.sources = HashMap::new();
                    return self.create_sources(block);
                }
            }
        }

        Ok(())
    }

    fn check_for_new_datasource(&mut self) -> Result<usize, SubgraphError> {
        let active_ds_count = self.sources.len();
        let all_ds_count = self.manifest.count_datasources();

        if active_ds_count > all_ds_count {
            // NOTE: templates are being used as un-addressed datasources
            return Ok(0);
        }

        let pending_ds = all_ds_count - active_ds_count;

        if pending_ds == 0 {
            return Ok(0);
        }

        let bundles = self.manifest.datasources_take_from(active_ds_count);
        assert_eq!(bundles.len(), pending_ds, "get latest ds failed");

        for ds in bundles {
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
        assert_eq!(self.sources.len(), all_ds_count, "adding datasource failed");
        Ok(pending_ds)
    }

    fn handle_ethereum_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        let block_number = block.number.as_u64();
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
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }

        let timer = Instant::now();
        let event_count = events.len();
        for event in events {
            let maybe_key1 = (
                event.datasource.clone(),
                Some(format!("{:?}", event.event.address).to_lowercase()),
            );
            let maybe_key2: (String, Option<String>) = (event.datasource.clone(), None);

            let mut maybe_source = self.sources.get_mut(&maybe_key1);

            if maybe_source.is_none() {
                maybe_source = self.sources.get_mut(&maybe_key2);
            }

            if maybe_source.is_none() {
                continue;
            }

            let source_instance = maybe_source.unwrap();
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

    pub fn process(&mut self, msg: FilteredDataMessage) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        self.metrics
            .current_block_number
            .set(block_ptr.number as i64);

        match msg {
            FilteredDataMessage::Ethereum { events, block } => {
                self.handle_ethereum_data(events, block)?
            }
        };

        if block_ptr.number % 1000 == 0 {
            info!(
                Subgraph,
                format!("finished processing block #{}", block_ptr.number)
            );
        }

        Ok(())
    }
}
