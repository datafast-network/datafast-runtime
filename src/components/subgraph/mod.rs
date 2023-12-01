mod datasource_wasm_instance;
mod metrics;

use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::Datasource;
use crate::common::HandlerTypes;
use crate::components::ManifestLoader;
use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::error;
use crate::errors::SubgraphError;
use crate::info;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::rpc_client::RpcAgent;
use crate::runtime::wasm_host::create_wasm_host;
use datasource_wasm_instance::DatasourceWasmInstance;
use metrics::SubgraphMetrics;
use prometheus::Registry;
use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
use std::collections::HashMap;
use std::time::Instant;

pub struct Subgraph {
    // NOTE: using IPFS might lead to subgraph-id using a hex/hash
    pub id: String,
    pub name: String,
    metrics: SubgraphMetrics,
    db: DatabaseAgent,
    rpc: RpcAgent,
    wasm_files: HashMap<String, Vec<u8>>,
    sources: Vec<Datasource>,
}

impl Subgraph {
    pub async fn new_empty(
        config: &Config,
        registry: &Registry,
        db: DatabaseAgent,
        rpc: RpcAgent,
        manifest: ManifestLoader,
    ) -> Self {
        let sources = manifest.datasources();
        let mut wasm_files = HashMap::new();
        for source in sources.iter() {
            let wasm_file = manifest.load_wasm(&source.name).await.unwrap();
            wasm_files.insert(source.name.clone(), wasm_file);
        }
        Self {
            name: config.subgraph_name.clone(),
            id: config.get_subgraph_id(),
            metrics: SubgraphMetrics::new(registry),
            db,
            rpc,
            wasm_files,
            sources,
        }
    }

    fn handle_ethereum_filtered_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut hosts = self
            .sources
            .par_iter()
            .map(|source| {
                let wasm_file = self.wasm_files.get(&source.name).unwrap().clone();
                let version = source.mapping.apiVersion.clone();
                let host = create_wasm_host(
                    version,
                    wasm_file,
                    self.db.clone(),
                    source.name.clone(),
                    self.rpc.clone(),
                    104_857_600,
                )
                .unwrap();
                let source_instance =
                    DatasourceWasmInstance::try_from((host, source.clone())).unwrap();
                (source.name.clone(), source_instance)
            })
            .collect::<HashMap<String, DatasourceWasmInstance>>();

        let mut block_handlers = HashMap::new();
        for source in self.sources.iter() {
            let source_instance = hosts.get_mut(&source.name).unwrap();
            let source_block_handlers = source_instance
                .ethereum_handlers
                .block
                .keys()
                .cloned()
                .collect::<Vec<String>>();
            block_handlers.insert(source.name.to_owned(), source_block_handlers);
        }

        for (source_name, ethereum_handlers) in block_handlers {
            // FIXME: this is not correct, block-handler may have filter itself,
            // thus not all datasource would handle the same block
            let source_instance = hosts.get_mut(&source_name).unwrap();
            for handler in ethereum_handlers {
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }
        for event in events {
            let source_instance = hosts.get_mut(&event.datasource).unwrap();
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

    pub async fn run_sync(
        &mut self,
        msg: FilteredDataMessage,
        db_agent: &DatabaseAgent,
        rpc_agent: &RpcAgent,
    ) -> Result<(), SubgraphError> {
        let block_ptr = msg.get_block_ptr();

        rpc_agent.set_block_ptr(block_ptr.clone()).await;

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
                "Finished processing block";
                block_number => block_ptr.number,
                block_hash => block_ptr.hash
            );
        }

        if block_ptr.number % 2000 == 0 {
            info!(Subgraph, "Committing data to DB"; block_number => block_ptr.number);
            let time = Instant::now();
            db_agent.migrate(block_ptr.clone()).await.map_err(|e| {
                error!(Subgraph, "Failed to commit db";
                       error => e.to_string(),
                       block_number => block_ptr.number,
                       block_hash => block_ptr.hash
                );
                SubgraphError::MigrateDbError
            })?;
            info!(Subgraph, "Db commit OK"; execution_time => format!("{:?}", time.elapsed()));
        }
        if block_ptr.number % 10000 == 0 {
            info!(Subgraph, "Clearing in-memory db"; block_number => block_ptr.number);
            db_agent
                .clear_in_memory()
                .await
                .map_err(|_| SubgraphError::MigrateDbError)?;
        }

        Ok(())
    }
}
