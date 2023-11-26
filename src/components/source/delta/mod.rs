mod ethereum;
mod utils;

use crate::config::DeltaConfig;
use crate::error;
use crate::errors::SourceError;
use crate::info;
use crate::messages::SerializedDataMessage;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::prelude::SessionContext;
pub use ethereum::DeltaEthereumBlocks;
use kanal::AsyncSender;
use std::sync::Arc;

pub trait DeltaBlockTrait: TryFrom<RecordBatch> + Into<Vec<SerializedDataMessage>> {}

pub struct DeltaClient {
    ctx: SessionContext,
    start_block: u64,
    query_step: u64,
}

impl DeltaClient {
    pub async fn new(cfg: DeltaConfig, start_block: u64) -> Result<Self, SourceError> {
        let ctx = SessionContext::new();
        let table = deltalake::open_table(&cfg.table_path).await?;
        ctx.register_table("blocks", Arc::new(table))?;
        Ok(Self {
            ctx,
            start_block,
            query_step: cfg.query_step,
        })
    }

    async fn query_arrow_records(&self, query: &str) -> Result<Vec<RecordBatch>, SourceError> {
        let batches = self.ctx.sql(query).await?.collect().await?;
        Ok(batches)
    }

    async fn get_blocks<R: DeltaBlockTrait>(
        &self,
        start_block: u64,
    ) -> Result<Vec<SerializedDataMessage>, SourceError> {
        let query = format!(
            "SELECT * FROM blocks WHERE block_number >= {start_block} AND block_number < {}",
            start_block + self.query_step
        );
        let batches = self
            .query_arrow_records(&query)
            .await?
            .first()
            .cloned()
            .ok_or_else(|| {
                error!(DeltaClient, "No blocks found");
                SourceError::DeltaEmptyData
            })?;
        let blocks = R::try_from(batches).map_err(|_| {
            error!(DeltaClient, "serialization to blocks failed");
            SourceError::DeltaSerializationError
        })?;
        let messages = Into::<Vec<SerializedDataMessage>>::into(blocks);
        Ok(messages)
    }

    pub async fn get_block_stream<R: DeltaBlockTrait>(
        self,
        sender: AsyncSender<Vec<SerializedDataMessage>>,
    ) -> Result<(), SourceError> {
        let mut start_block = self.start_block;

        while let Ok(blocks) = self.get_blocks::<R>(start_block).await {
            let batch_first = blocks.first().expect("no block returned").get_block_ptr();
            let batch_last = blocks.last().expect("no block returned").get_block_ptr();
            info!(DeltaClient, "new block range returned"; first => batch_first, last => batch_last);
            start_block = batch_last.number + 1;
            sender.send(blocks).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_delta() {
        env_logger::try_init().unwrap_or_default();

        let cfg = DeltaConfig {
            table_path: "s3://blocks/ethereum/".to_owned(),
            query_step: 100,
        };

        let client = DeltaClient::new(cfg, 10_000_000).await.unwrap();
        let blocks = client
            .get_blocks::<DeltaEthereumBlocks>(10_000_000)
            .await
            .unwrap();

        assert_eq!(blocks.len(), 200);

        for (idx, block) in blocks.into_iter().enumerate() {
            assert_eq!(10_000_000 + idx, block.get_block_ptr().number as usize);
            let _msg = block;
        }
    }
}
