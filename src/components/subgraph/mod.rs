mod datasource_wasm_instance;
mod metrics;

use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::Datasource;
use crate::common::HandlerTypes;
use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::error;
use crate::errors::SubgraphError;
use crate::info;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::rpc_client::RpcAgent;
use crate::runtime::wasm_host::AscHost;
use datasource_wasm_instance::DatasourceWasmInstance;
use kanal::AsyncReceiver;
use metrics::SubgraphMetrics;
use prometheus::Registry;
use std::collections::HashMap;

pub struct Subgraph {
    // NOTE: using IPFS might lead to subgraph-id using a hex/hash
    pub id: String,
    pub name: String,
    sources: HashMap<String, DatasourceWasmInstance>,
    metrics: SubgraphMetrics,
}

impl Subgraph {
    pub fn new_empty(config: &Config, registry: &Registry) -> Self {
        Self {
            sources: HashMap::new(),
            name: config.subgraph_name.clone(),
            id: config.get_subgraph_id(),
            metrics: SubgraphMetrics::new(registry),
        }
    }

    pub fn create_source(
        &mut self,
        host: AscHost,
        datasource: Datasource,
    ) -> Result<(), SubgraphError> {
        let source = DatasourceWasmInstance::try_from((host, datasource))?;
        self.sources.insert(source.id.clone(), source);
        Ok(())
    }

    async fn handle_ethereum_filtered_data(
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
        let start = tokio::time::Instant::now();
        for event in events {
            let source_instance = self
                .sources
                .get_mut(&event.datasource)
                .ok_or(SubgraphError::InvalidSourceID(event.datasource.to_owned()))?;
            source_instance.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;
        }
        info!(
            Subgraph,
            "Handled ethereum filtered data";
            time => format!("{:?}", start.elapsed())
        );
        Ok(())
    }

    async fn handle_filtered_data(
        &mut self,
        data: FilteredDataMessage,
    ) -> Result<(), SubgraphError> {
        match data {
            FilteredDataMessage::Ethereum { events, block } => {
                let start = tokio::time::Instant::now();
                let event_len = events.len();
                let block_number = block.number;
                self.handle_ethereum_filtered_data(events, block).await?;
                info!(
                    Subgraph,
                    ">>>>>> Processed events in block";
                    events => event_len,
                    block => format!("{:?}", block_number),
                    time => format!("{:?}", start.elapsed())
                );
                Ok(())
            }
        }
    }

    pub async fn run_async(
        mut self,
        recv: AsyncReceiver<FilteredDataMessage>,
        db_agent: DatabaseAgent,
        rpc_agent: RpcAgent,
    ) -> Result<(), SubgraphError> {
        while let Ok(msg) = recv.recv().await {
            let block_ptr = msg.get_block_ptr();

            rpc_agent.set_block_ptr(block_ptr.clone()).await;

            self.metrics
                .current_block_number
                .set(block_ptr.number as i64);

            let timer = self.metrics.block_process_duration.start_timer();
            self.handle_filtered_data(msg).await?;
            timer.stop_and_record();
            self.metrics.block_process_counter.inc();
            let start = tokio::time::Instant::now();
            db_agent.migrate(block_ptr.clone()).await.map_err(|e| {
                error!(Subgraph, "Failed to migrate db";
                    error => e.to_string(),
                    block_number => block_ptr.number,
                    block_hash => block_ptr.hash
                );
                SubgraphError::MigrateDbError
            })?;

            db_agent
                .clear_in_memory()
                .await
                .map_err(|_| SubgraphError::MigrateDbError)?;
            info!(
                Subgraph,
                ">>>>>> Migrated db";
                block_number => block_ptr.number,
                block_hash => block_ptr.hash,
                time => format!("{:?}", start.elapsed())
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::datasource_wasm_instance::DatasourceWasmInstance;
    use super::datasource_wasm_instance::EthereumHandlers;
    use super::datasource_wasm_instance::Handler;
    use super::metrics::SubgraphMetrics;
    use super::Subgraph;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::chain::ethereum::event::EthereumEventData;
    use crate::database::DatabaseAgent;
    use crate::messages::EthereumFilteredEvent;
    use crate::messages::FilteredDataMessage;
    use crate::rpc_client::RpcAgent;
    use crate::runtime::wasm_host::test::get_subgraph_testing_resource;
    use crate::runtime::wasm_host::test::mock_wasm_host;
    use async_std::task;
    use prometheus::default_registry;
    use std::collections::HashMap;

    #[::rstest::rstest]
    #[case("0.0.4")]
    #[case("0.0.5")]
    async fn test_subgraph(#[case] version: &str) {
        env_logger::try_init().unwrap_or_default();
        let registry = default_registry();

        let mut subgraph = Subgraph {
            id: "TestSubgraph".to_string(),
            name: "TestSubgraph".to_string(),
            sources: HashMap::new(),
            metrics: SubgraphMetrics::new(registry),
        };

        let subgraph_sources = vec!["TestDataSource1"];

        for source_name in subgraph_sources {
            let (version, wasm_path) = get_subgraph_testing_resource(version, "TestDataSource");

            let id = source_name.to_string();
            let host = mock_wasm_host(version.clone(), &wasm_path, registry, RpcAgent::new_mock());
            let mut ethereum_handlers = EthereumHandlers {
                block: HashMap::new(),
                events: HashMap::new(),
            };

            ethereum_handlers.block.insert(
                "testHandlerBlock".to_owned(),
                Handler::new(&host.instance.exports, "testHandlerBlock").unwrap(),
            );
            ethereum_handlers.events.insert(
                "testHandlerEvent".to_owned(),
                Handler::new(&host.instance.exports, "testHandlerEvent").unwrap(),
            );

            subgraph.sources.insert(
                source_name.to_string(),
                DatasourceWasmInstance {
                    id,
                    host,
                    ethereum_handlers,
                },
            );
        }

        log::info!("Finished setup");

        let (sender, receiver) = kanal::bounded_async(1);
        let agent = DatabaseAgent::empty(registry);
        let rpc_agent = RpcAgent::new_mock();
        let t = task::spawn(subgraph.run_async(receiver, agent, rpc_agent));

        // Test sending block data
        let block_data_msg = FilteredDataMessage::Ethereum {
            events: vec![],
            block: EthereumBlockData::default(),
        };
        log::info!("------- Send block to blockHandler of Subgraph");
        sender
            .send(block_data_msg)
            .await
            .expect("Failed to send block_data_msg");

        // Test sending event data
        let example_event = EthereumEventData {
            block: EthereumBlockData {
                number: ethabi::ethereum_types::U64::from(1000),
                ..Default::default()
            },
            ..Default::default()
        };
        let event_data_msg = FilteredDataMessage::Ethereum {
            events: vec![EthereumFilteredEvent {
                datasource: "TestDataSource1".to_string(),
                handler: "testHandlerEvent".to_string(),
                event: example_event,
            }],
            block: EthereumBlockData::default(),
        };
        log::info!("------- Send event to eventHandler of Subgraph");
        sender
            .send(event_data_msg)
            .await
            .expect("Failed to send event_data_msg");

        task::sleep(std::time::Duration::from_secs(2)).await;
        sender.close();
        t.await.unwrap();
    }
}
