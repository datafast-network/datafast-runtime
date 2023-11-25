pub mod ethereum;
mod utils;

use crate::config::TrinoConfig;
use crate::error;
use crate::errors::SourceError;
use crate::info;
use crate::messages::SerializedDataMessage;
pub use ethereum::TrinoEthereumBlock;
use kanal::AsyncSender;
use prusto::Client;
use prusto::ClientBuilder;
use prusto::Row;
use std::collections::HashSet;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

pub trait TrinoBlockTrait: TryFrom<Row> + Into<SerializedDataMessage> {
    fn get_block_number(&self) -> u64;
    fn get_block_hash(&self) -> String;
    fn get_parent_hash(&self) -> String;
    fn get_insert_timestamp(&self) -> u64;
}

pub struct TrinoClient {
    client: Client,
    start_block: u64,
    query_step: u64,
    table: String,
}

impl TrinoClient {
    pub fn new(cfg: TrinoConfig, start_block: u64) -> Result<Self, SourceError> {
        let client = ClientBuilder::new(cfg.user, cfg.host)
            .port(cfg.port)
            .catalog(cfg.catalog)
            .schema(cfg.schema)
            .build()
            .map_err(|e| {
                error!(TrinoClient, "Connection failed"; error => e);
                SourceError::TrinoConnectionFail
            })?;
        Ok(Self {
            client,
            start_block,
            query_step: cfg.query_step,
            table: cfg.table,
        })
    }

    async fn query(&self, query: &str) -> Result<Vec<Row>, SourceError> {
        Ok(self
            .client
            .get_all::<Row>(query.to_owned())
            .await
            .map_err(|e| {
                error!(TrinoClient, "Query failed"; error => e);
                SourceError::TrinoQueryFail
            })?
            .into_vec())
    }

    /* FIXME: this logic is incomplete
    - 1/ must have a way to check block-continuity
    - 2/ must know what block to resume pulling when restart
    - 3/ must have a way to dedup
     */
    async fn get_blocks<R: TrinoBlockTrait>(
        &self,
        start_block: u64,
    ) -> Result<Vec<SerializedDataMessage>, SourceError> {
        let query = format!(
            "SELECT * FROM {} WHERE block_number >= {} AND block_number < {}",
            self.table,
            start_block,
            start_block + self.query_step
        );
        let retry_strategy = FixedInterval::from_millis(1);
        let results = Retry::spawn(retry_strategy, || self.query(&query)).await?;

        let mut blocks = Vec::new();
        let mut block_hashes = HashSet::new();

        for row in results {
            let block = R::try_from(row).map_err(|_| SourceError::TrinoSerializeFail)?;
            let hash = block.get_block_hash();

            if !block_hashes.contains(&hash) {
                blocks.push(Into::<SerializedDataMessage>::into(block));
                block_hashes.insert(hash);
            }
        }

        blocks.sort_by_key(|b| b.get_block_ptr().number);

        Ok(blocks)
    }

    pub async fn get_block_stream<R: TrinoBlockTrait>(
        self,
        sender: AsyncSender<Vec<SerializedDataMessage>>,
    ) -> Result<(), SourceError> {
        let mut start_block = self.start_block;

        while let Ok(blocks) = self.get_blocks::<R>(start_block).await {
            let batch_first = blocks.first().expect("no block returned").get_block_ptr();
            let batch_last = blocks.last().expect("no block returned").get_block_ptr();
            info!(TrinoClient, "new block range returned"; first => batch_first, last => batch_last);
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
    async fn test_trino() {
        env_logger::try_init().unwrap_or_default();

        let cfg = TrinoConfig {
            host: "194.233.82.254".to_owned(),
            port: 8080,
            schema: "ethereum".to_string(),
            table: "ethereum".to_string(),
            query_step: 200,
            catalog: "delta".to_string(),
            user: "trino".to_string(),
        };

        let trino = TrinoClient::new(cfg, 1).unwrap();
        let blocks = trino
            .get_blocks::<TrinoEthereumBlock>(10_000_000)
            .await
            .unwrap();

        assert_eq!(blocks.len(), 200);

        for (idx, block) in blocks.into_iter().enumerate() {
            assert_eq!(10_000_000 + idx, block.get_block_ptr().number as usize);
            let _msg = block;
        }
    }
}
