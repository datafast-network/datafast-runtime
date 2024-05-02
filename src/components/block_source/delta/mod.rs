mod ethereum;

use super::metrics::BlockSourceMetrics;
use crate::common::BlockDataMessage;
use crate::components::Valve;
use crate::config::DeltaConfig;
use crate::errors::SourceError;
use crate::info;
use crate::warn;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::prelude::DataFrame;
use deltalake::datafusion::prelude::SessionContext;
pub use ethereum::DeltaEthereumBlocks;
use kanal::AsyncSender;
use prometheus::Registry;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use rayon::slice::ParallelSliceMut;
use std::sync::Arc;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

pub trait DeltaBlockTrait:
    TryFrom<RecordBatch, Error = SourceError> + Into<Vec<BlockDataMessage>>
{
}

pub struct DeltaClient {
    ctx: SessionContext,
    start_block: u64,
    query_step: u64,
    metrics: BlockSourceMetrics,
}

impl DeltaClient {
    pub async fn new(
        cfg: DeltaConfig,
        start_block: u64,
        registry: &Registry,
    ) -> Result<Self, SourceError> {
        info!(
            DeltaClient,
            "Init connection to data store";
            version => format!("{:?}", cfg.version),
            path => cfg.table_path
        );
        let ctx = SessionContext::new();
        let table = match cfg.version {
            None => deltalake::open_table(&cfg.table_path).await?,
            Some(version) => {
                deltalake::open_table_with_version(&cfg.table_path, version as i64).await?
            }
        };
        let file_count = table.get_files_count();
        ctx.register_table("blocks", Arc::new(table))?;
        info!(
            DeltaClient,
            "Setup done";
            query_step => cfg.query_step,
            start_block => start_block,
            table_path => cfg.table_path,
            version => cfg.version.map(|v| v.to_string()).unwrap_or("latest".to_string()),
            file_count => file_count
        );
        Ok(Self {
            ctx,
            start_block,
            query_step: cfg.query_step,
            metrics: BlockSourceMetrics::new(registry),
        })
    }

    async fn get_dataframe(&self, query: &str) -> Result<DataFrame, SourceError> {
        let df = self.ctx.sql(query).await?;
        Ok(df)
    }

    async fn query_blocks(&self, start_block: u64) -> Result<Vec<RecordBatch>, SourceError> {
        let query = format!(
            "SELECT block_data FROM blocks WHERE block_number >= {} AND block_number < {}",
            start_block,
            start_block + self.query_step
        );
        let start_time = self.metrics.block_source_query_duration.start_timer();
        let df = self.get_dataframe(&query).await?;
        info!(BlockSource, "dataframe set up OK"; query => query);
        let batches = df.collect().await?;
        start_time.stop_and_record();
        self.metrics.block_source_query_count.inc();
        Ok(batches)
    }

    pub async fn get_block_stream<R: DeltaBlockTrait>(
        &self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
        valve: Valve,
    ) -> Result<(), SourceError> {
        let mut start_block = self.start_block;
        info!(BlockSource, "start polling for block-data ⚓");

        loop {
            let batches = Retry::spawn(FixedInterval::from_millis(10), || {
                self.query_blocks(start_block)
            })
            .await?;

            let start_time = self.metrics.block_source_serialized_duration.start_timer();

            let mut blocks = batches
                .into_par_iter()
                .flat_map(|batch| {
                    let blocks = R::try_from(batch).unwrap();

                    Into::<Vec<BlockDataMessage>>::into(blocks)
                })
                .collect::<Vec<_>>();

            blocks.par_sort_unstable_by_key(|b| b.get_block_ptr().number);

            start_time.stop_and_record();

            self.metrics
                .block_source_total_blocks
                .inc_by(blocks.len() as u64);

            info!(
                DeltaClient,
                "block batch serialization finished";
                number_of_blocks => blocks.len()
            );

            if blocks.is_empty() {
                warn!(BlockSource, "No more block to query...");
                return Ok(());
            }

            valve.set_downloaded(blocks.last().map(|b| b.get_block_ptr().number).unwrap());
            sender.send(blocks).await?;
            start_block += self.query_step;
            valve.temporarily_close().await;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::ValveConfig;
    use log::info;
    use prometheus::default_registry;
    use serde_json::json;

    #[test]
    fn test_adjust_start_block() {
        let actual_start_block = 10_124_125;
        let block_per_file = 2000;
        let adjusted_start_block = actual_start_block - (actual_start_block % block_per_file);
        assert_eq!(adjusted_start_block, 10_124_000);
    }

    #[tokio::test]
    async fn test_delta() {
        env_logger::try_init().unwrap_or_default();

        let cfg = DeltaConfig {
            table_path: "s3://ethereum/blocks_proto/".to_owned(),
            query_step: 4000,
            version: None,
        };
        let registry = default_registry();
        let client = DeltaClient::new(cfg, 10_000_000, registry).await.unwrap();
        let (sender, recv) = kanal::bounded_async(1);

        tokio::select! {
            _ = client.get_block_stream::<DeltaEthereumBlocks>(sender, Valve::new(&ValveConfig::default(), registry)) => {
                log::info!(" DONE SENDER");
            },
            _ = async move {
                while let Ok(b) = recv.recv().await {
                    let first = b.first().map(|f| f.get_block_ptr()).unwrap();
                    let last = b.last().map(|f| f.get_block_ptr()).unwrap();
                    log::warn!("Received: {:?} msgs, first_block={:#?}, last_block={:#?}", b.len(), first, last);
                    recv.close();
                }
            } => {
                log::info!(" DONE RECV");
            }
        }
    }

    #[tokio::test]
    async fn test_ethereum_serialization() {
        env_logger::try_init().unwrap_or_default();

        let cfg = DeltaConfig {
            table_path: "s3://ethereum/blocks_proto/".to_owned(),
            query_step: 1,
            version: None,
        };

        let client = DeltaClient::new(cfg, 10_000_000, default_registry())
            .await
            .unwrap();

        let (sender, recv) = kanal::bounded_async::<Vec<BlockDataMessage>>(1);

        let assert_block = async move {
            while let Ok(b) = recv.recv().await {
                let block = b.first().unwrap();

                let BlockDataMessage::Ethereum {
                    block,
                    transactions,
                    logs,
                } = block;

                info!("Validating block header...");
                assert_eq!(
                    format!("{:?}", block.hash),
                    "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe"
                );
                assert_eq!(
                    format!("{:?}", block.parent_hash),
                    "0x966bf6849da92ff2a0e3db9a371f5b9f07dd6001e2770a4269a5c134f1bf9c4c"
                );
                assert_eq!(
                    format!("{:?}", block.state_root),
                    "0x74477eaabece6bce00c346dc12275b2ed74ec9d6c758c4023c2040ba0e72e05d"
                );

                assert_eq!(block.number.as_u64(), 10_000_000);

                assert_eq!(transactions.len(), 103);

                // -------------- CHECK Transactions
                let first_tx = transactions.first().cloned().unwrap();
                let last_tx = transactions.last().cloned().unwrap();

                // ------------------- First tx
                info!("Validating first Tx in block...");
                assert_eq!(first_tx.index.as_u64(), 0);
                assert_eq!(first_tx.nonce.as_u64(), 25936206);
                assert_eq!(
                    format!("{:?}", first_tx.hash),
                    "0x4a1e3e3a2aa4aa79a777d0ae3e2c3a6de158226134123f6c14334964c6ec70cf"
                );
                assert_eq!(first_tx.value.to_string(), "384134310464384681");
                assert_eq!(
                    format!("{:?}", first_tx.from),
                    "0xea674fdde714fd979de3edf0f56aa9716b898ec8"
                );
                assert_eq!(
                    format!("{:?}", first_tx.to.unwrap()),
                    "0x60f18d941f6253e3f7082ea0db3bc3944e7e9d40"
                );

                // ---------- Last TX
                info!("Validating last Tx in block...");
                assert_eq!(last_tx.index.as_u64(), 102);
                assert_eq!(last_tx.nonce.as_u64(), 47);
                assert_eq!(
                    format!("{:?}", last_tx.hash),
                    "0x5a4bf6970980a9381e6d6c78d96ab278035bbff58c383ffe96a0a2bbc7c02a4b"
                );
                assert_eq!(last_tx.value.to_string(), "2000000000000000000");
                assert_eq!(
                    format!("{:?}", last_tx.from),
                    "0x8a9d69aa686fa0f9bbdec21294f67d4d9cfb4a3e"
                );
                assert_eq!(
                    format!("{:?}", last_tx.to.unwrap()),
                    "0xd69b8ff1888e78d9c337c2f2e6b3bf3e7357800e"
                );

                // ---------- Logs
                info!("Validating logs...");
                assert_eq!(logs.len(), 135);
                let log = serde_json::to_value(&logs[0].clone())
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .to_owned();
                let expected_log = json!({
                        "address": "0xced4e93198734ddaff8492d525bd258d49eb388e",
                        "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
                        "blockNumber": "0x989680",
                        "data": "0x0000000000000000000000000000000000000000000000052769477a7d940000",
                        "logIndex": "0x0",
                        "removed": false,
                        "topics": [
                            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                            "0x000000000000000000000000876eabf441b2ee5b5b0554fd502a8e0600950cfa",
                            "0x000000000000000000000000566021352eb2f882538bf8d59e5d2ba741b9ec7a"
                        ],
                        "transactionHash": "0x1f17943d5dd7053959f1dc092dfad60a7caa084224212b1adbecaf3137efdfdd",
                        "transactionIndex": "0x5",
                        "logType": null,
                        "transactionLogIndex": null
                    }).as_object().unwrap().to_owned();

                for key in log.keys() {
                    assert_eq!(log.get(key), expected_log.get(key),);
                }

                let log = serde_json::to_value(logs.last())
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .to_owned();
                let expected_log = json!({
                    "address": "0xdac17f958d2ee523a2206206994597c13d831ec7",
                    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
                    "blockNumber": "0x989680",
                    "data": "0x000000000000000000000000000000000000000000000000000000009f280a06",
                    "logIndex": "0x86",
                    "removed": false,
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                        "0x0000000000000000000000009c43d6a63f7fd0ae469ec0aeda2a90be93038f59",
                        "0x00000000000000000000000096e9a06a22d4445a757dfe9b4ff2c77a12dd60f2"
                    ],
                    "transactionHash": "0xf9084755ea9905d54a61b1109626ad3de5e8c2edf3b9f7a42831037ace6f2456",
                    "transactionIndex": "0x63",
                    "logType": null,
                    "transactionLogIndex": null
                }).as_object().unwrap().to_owned();

                for key in log.keys() {
                    assert_eq!(log.get(key), expected_log.get(key),);
                }

                recv.close();
            }
        };

        tokio::select! {
            _ = client.get_block_stream::<DeltaEthereumBlocks>(sender, Valve::new(&ValveConfig::default(), &Registry::default())) => (),
            _ = assert_block => ()
        }
    }
}
