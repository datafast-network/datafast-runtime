mod ethereum;

use crate::components::Valve;
use crate::config::DeltaConfig;
use crate::errors::SourceError;
use crate::info;
use crate::messages::BlockDataMessage;
use crate::warn;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::prelude::DataFrame;
use deltalake::datafusion::prelude::SessionContext;
pub use ethereum::DeltaEthereumBlocks;
use kanal::AsyncSender;
use std::sync::Arc;
use std::time::Duration;

pub trait DeltaBlockTrait:
    TryFrom<RecordBatch, Error = SourceError> + Into<Vec<BlockDataMessage>>
{
}

pub struct DeltaClient {
    ctx: SessionContext,
    start_block: u64,
    query_step: u64,
}

impl DeltaClient {
    pub async fn new(cfg: DeltaConfig, start_block: u64) -> Result<Self, SourceError> {
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
        let file_count = table.get_files().len();
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
        })
    }

    async fn get_dataframe(&self, query: &str) -> Result<DataFrame, SourceError> {
        let df = self.ctx.sql(query).await?;
        Ok(df)
    }

    async fn query_blocks(&self, start_block: u64) -> Result<Vec<RecordBatch>, SourceError> {
        let query = format!(
            "SELECT * FROM blocks WHERE block_number >= {} AND block_number < {}",
            start_block,
            start_block + self.query_step
        );
        let df = self.get_dataframe(&query).await?;
        info!(DeltaClient, "dataframe set up OK"; query => query);
        let batches = df.collect().await?;
        Ok(batches)
    }

    pub async fn get_block_stream<R: DeltaBlockTrait>(
        &self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
        valve: Valve,
    ) -> Result<(), SourceError> {
        let mut start_block = self.start_block;
        info!(DeltaClient, "source start collecting data");

        loop {
            while !valve.should_continue() {
                let sleep_tine = valve.get_wait();
                info!(DeltaClient, "source sleeping"; sleep_time => sleep_tine);
                tokio::time::sleep(Duration::from_secs(sleep_tine)).await;
            }

            let mut collect_msg = vec![];

            for batch in self.query_blocks(start_block).await? {
                let time = std::time::Instant::now();
                let blocks = R::try_from(batch)?;
                let messages = Into::<Vec<BlockDataMessage>>::into(blocks);
                valve.set_downloaded(&messages);

                info!(
                    DeltaClient,
                    "batches received & serialized";
                    serialize_time => format!("{:?}", time.elapsed()),
                    number_of_blocks => messages.len()
                );

                collect_msg.extend(messages);
            }

            if collect_msg.is_empty() {
                warn!(DeltaClient, "No more block to query...");
                return Ok(());
            }

            sender.send(collect_msg).await?;
            start_block += self.query_step;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::ValveConfig;

    #[tokio::test]
    async fn test_delta() {
        env_logger::try_init().unwrap_or_default();

        let cfg = DeltaConfig {
            table_path: "s3://ethereum/blocks_01/".to_owned(),
            query_step: 10,
            version: None,
        };

        let client = DeltaClient::new(cfg, 10_000_000).await.unwrap();
        let (sender, recv) = kanal::bounded_async(1);

        tokio::select! {
            _ = client.get_block_stream::<DeltaEthereumBlocks>(sender, Valve::new(&ValveConfig::default())) => {
                log::info!(" DONE SENDER");
            },
            _ = async move {
                while let Ok(b) = recv.recv().await {
                    let first = b.first().map(|f| f.get_block_ptr()).unwrap();
                    let last = b.last().map(|f| f.get_block_ptr()).unwrap();
                    log::warn!("Received: {:?} msgs, first_block={}, last_block={}", b.len(), first, last);
                    recv.close();
                }
            } => {
                log::info!(" DONE RECV");
            }
        }
    }
}
