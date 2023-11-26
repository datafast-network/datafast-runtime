mod ethereum;

use crate::config::DeltaConfig;
use crate::debug;
use crate::errors::SourceError;
use crate::info;
use crate::messages::SerializedDataMessage;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::physical_plan::SendableRecordBatchStream;
use deltalake::datafusion::prelude::DataFrame;
use deltalake::datafusion::prelude::SessionContext;
pub use ethereum::DeltaEthereumBlocks;
use kanal::AsyncSender;
use std::sync::Arc;
use tokio_stream::StreamExt;

pub trait DeltaBlockTrait:
    TryFrom<RecordBatch, Error = SourceError> + Into<Vec<SerializedDataMessage>>
{
}

pub struct DeltaClient {
    ctx: SessionContext,
    start_block: u64,
    query_step: u64,
}

impl DeltaClient {
    pub async fn new(cfg: DeltaConfig, start_block: u64) -> Result<Self, SourceError> {
        info!(DeltaClient, "Init connection to data store");
        let ctx = SessionContext::new();
        let table = deltalake::open_table(&cfg.table_path).await?;
        info!(DeltaClient, "Table found OK");
        ctx.register_table("blocks", Arc::new(table))?;
        info!(DeltaClient, "Registered table");
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

    async fn query_blocks(
        &self,
        start_block: u64,
    ) -> Result<SendableRecordBatchStream, SourceError> {
        let query = format!(
            "SELECT * FROM blocks WHERE block_number >= {} AND block_number < {} ORDER BY block_number ASC",
            start_block,
            start_block + self.query_step
        );
        let df = self.get_dataframe(&query).await?;
        debug!(DeltaClient, "dataframe set up OK"; query => query);
        let stream = df.execute_stream().await?;
        Ok(stream)
    }

    pub async fn get_block_stream<R: DeltaBlockTrait>(
        &self,
        sender: AsyncSender<Vec<SerializedDataMessage>>,
    ) -> Result<(), SourceError> {
        let mut start_block = self.start_block;

        loop {
            let mut stream = self.query_blocks(start_block).await?;

            while let Some(Ok(batches)) = stream.next().await {
                let time = std::time::Instant::now();
                let blocks = R::try_from(batches)?;
                let messages = Into::<Vec<SerializedDataMessage>>::into(blocks);
                info!(
                    DeltaClient,
                    "batches received & serialized";
                    serialize_time => format!("{:?}", time.elapsed()),
                    start_block => start_block,
                    last_block => format!("{:?}", start_block + self.query_step),
                    number_of_blocks => messages.len()
                );
                sender.send(messages).await?;
                info!(DeltaClient, "block batches sent");
            }

            start_block += self.query_step;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_delta() {
        env_logger::try_init().unwrap_or_default();

        let cfg = DeltaConfig {
            table_path: "s3://ethereum/".to_owned(),
            query_step: 1000,
        };

        let client = DeltaClient::new(cfg, 10_000_000).await.unwrap();
        let (sender, recv) = kanal::bounded_async(1);

        tokio::select! {
            _ = client.get_block_stream::<DeltaEthereumBlocks>(sender) => {
                log::info!(" DONE SENDER");
            },
            _ = async move {
                while let Ok(b) = recv.recv().await {
                    let first = b.first().map(|f| f.get_block_ptr()).unwrap();
                    let last = b.last().map(|f| f.get_block_ptr()).unwrap();
                    log::warn!("Received: {:?} msgs, first_block={}, last_block={}", b.len(), first, last);
                }
            } => {
                log::info!(" DONE RECV");
            }
        }
    }
}
