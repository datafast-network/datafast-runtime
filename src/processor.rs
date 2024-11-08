use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::errors::MainError;
use crate::rpc_client::RpcAgent;
use crate::BlockInspectionResult;
use crate::BlockSource;
use crate::DataFilter;
use crate::Inspector;
use crate::ManifestAgent;
use crate::Subgraph;
use crate::Valve;
use df_logger::*;
use prometheus::Registry;
use std::sync::RwLock;

pub struct Processor {
    config: Config,
    valve: Valve,
    manifest: ManifestAgent,
    db: DatabaseAgent,
    rpc: RwLock<RpcAgent>,
    inspector: RwLock<Inspector>,
    block_source: BlockSource,
    filter: DataFilter,
    subgraph: RwLock<Subgraph>,
}

impl Processor {
    pub async fn new(
        config: Config,
        registry: &Registry,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let manifest = ManifestAgent::new(&config.subgraph_dir).await?;
        info!(main, "Manifest loaded!");

        let valve = Valve::new(&config.valve, registry);

        let db = DatabaseAgent::new(&config.database, manifest.schemas(), registry).await?;
        info!(main, "Database ready!");

        let inspector = Inspector::new(
            db.get_recent_block_pointers(config.reorg_threshold).await?,
            manifest.min_start_block(),
            config.reorg_threshold,
        );
        info!(main, "BlockInspector ready!"; next_start_block => inspector.get_expected_block_number());

        let block_source =
            BlockSource::new(&config, inspector.get_expected_block_number(), registry).await?;
        info!(main, "BlockSource ready!");

        let filter = DataFilter::new(
            config.chain.clone(),
            manifest.datasource_and_templates().into(),
            manifest.abis(),
        )?;
        info!(main, "DataFilter ready!");

        let rpc = RpcAgent::new(&config, manifest.abis(), registry).await?;
        info!(main, "Rpc-Client ready!");

        let subgraph = Subgraph::new(&db, &rpc, &manifest, registry);
        info!(main, "Subgraph ready!");

        let this = Self {
            config,
            valve,
            manifest,
            db,
            rpc: RwLock::new(rpc),
            inspector: RwLock::new(inspector),
            block_source,
            filter,
            subgraph: RwLock::new(subgraph),
        };

        Ok(this)
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let source_valve = self.valve.clone();

        let (sender, recv) = kanal::bounded_async(1);

        let query_blocks = async move {
            self.block_source
                .run(sender, source_valve)
                .await
                .map_err(MainError::from)
        };

        let mut subgraph = self.subgraph.write().expect("Lock-write subgraph");
        let mut rpc = self.rpc.write().expect("Lock-write rpc");
        let mut inspector = self.inspector.write().expect("Lock-write inspector");

        subgraph.create_sources()?;

        let main_flow = async move {
            while let Ok(blocks) = recv.recv().await {
                info!(
                    main,
                    "block batch recevied and about to be processed 🚀";
                    total_block => blocks.len()
                );

                let time = std::time::Instant::now();
                let blocks = self.filter.filter_multi(blocks)?;
                let count_blocks = blocks.len();
                let last_block = blocks.last().map(|b| b.get_block_ptr()).unwrap();

                info!(
                    main,
                    "data scanned & filtered 🔎";
                    exec_time => format!("{:?}", time.elapsed()),
                    count_blocks => count_blocks
                );

                let time = std::time::Instant::now();

                for block in blocks {
                    let block_ptr = block.get_block_ptr();
                    rpc.set_block_ptr(&block_ptr);
                    self.manifest.set_block_ptr(&block_ptr);

                    match inspector.check_block(block_ptr.clone()) {
                        BlockInspectionResult::UnexpectedBlock
                        | BlockInspectionResult::UnrecognizedBlock => {
                            panic!("Bad block data from source");
                        }
                        BlockInspectionResult::BlockAlreadyProcessed
                        | BlockInspectionResult::MaybeReorg => {
                            continue;
                        }
                        BlockInspectionResult::ForkBlock => {
                            self.db.revert_from_block(block_ptr.number).await?;
                        }
                        BlockInspectionResult::OkToProceed => (),
                    };

                    if subgraph.should_process(&block) {
                        subgraph.process(block)?;
                        rpc.clear_block_level_cache();
                    }

                    self.valve.set_finished(block_ptr.number);
                }

                let elapsed = time.elapsed();

                self.db.commit_data(last_block.clone()).await?;
                self.db.remove_outdated_snapshots(last_block.number).await?;
                self.db.flush_cache().await?;

                if let Some(history_size) = self.config.block_data_retention {
                    if last_block.number > history_size {
                        self.db
                            .clean_data_history(last_block.number - history_size)
                            .await?;
                    }
                }

                info!(
                    main,
                    "BLOCK BATCH PROCESSED DONE  🎉🎉🎉🎉";
                    exec_time => format!("{:?}", elapsed),
                    number_of_blocks => count_blocks,
                    avg_speed => format!("~{:?} blocks/sec", { count_blocks as u64 / elapsed.as_secs() })
                );
            }

            warn!(main, "No more messages returned from block-stream");
            Ok::<(), MainError>(())
        };

        let result = tokio::try_join! {
            query_blocks,
            main_flow
        };

        info!(main, format!("Processor has finished"); result => format!("{:?}", result));

        Ok(())
    }
}
