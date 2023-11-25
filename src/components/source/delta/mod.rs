mod ethereum;

use crate::config::DeltaConfig;
use crate::error;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::prelude::SessionContext;
use std::sync::Arc;

pub trait DeltaBlockTrait: TryFrom<Vec<RecordBatch>> + Into<Vec<SerializedDataMessage>> {}

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
        let batches = self.query_arrow_records(&query).await?;
        let blocks = R::try_from(batches).map_err(|_| {
            error!(DeltaClient, "serialization to blocks failed");
            SourceError::DeltaSerializationError
        })?;
        let messages = Into::<Vec<SerializedDataMessage>>::into(blocks);
        Ok(messages)
    }
}
