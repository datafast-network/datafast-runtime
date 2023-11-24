mod ethereum;
mod utils;

use crate::error;
use crate::errors::SourceError;
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
}

impl TrinoClient {
    pub fn new(
        host: &str,
        port: &u16,
        user: &str,
        catalog: &str,
        schema: &str,
        start_block: u64,
        query_step: u64,
    ) -> Result<Self, SourceError> {
        let client = ClientBuilder::new(user, host)
            .port(port.to_owned())
            .catalog(catalog)
            .schema(schema)
            .build()
            .unwrap();
        Ok(Self {
            client,
            start_block,
            query_step,
        })
    }

    async fn query(&self, query: &str) -> Result<Vec<Row>, SourceError> {
        Ok(self
            .client
            .get_all::<Row>(query.to_owned())
            .await
            .map_err(|e| {
                error!(TrinoClient, "query failed"; error => format!("{:?}", e));
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
        stop_block: u64,
    ) -> Result<Vec<SerializedDataMessage>, SourceError> {
        let query = format!(
            r#"
SELECT * FROM blocks_2
WHERE block_number >= {} AND block_number < {}
"#,
            start_block, stop_block
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
        let mut blocks = self
            .get_blocks::<R>(start_block, start_block + self.query_step)
            .await
            .unwrap()
            .into_iter()
            .map(Into::<SerializedDataMessage>::into)
            .collect::<Vec<_>>();

        loop {
            sender.send(blocks).await?;
            start_block += self.query_step;
            blocks = self
                .get_blocks::<R>(start_block, start_block + self.query_step)
                .await
                .unwrap()
                .into_iter()
                .map(Into::<SerializedDataMessage>::into)
                .collect::<Vec<_>>();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_trino() {
        env_logger::try_init().unwrap_or_default();

        let trino =
            TrinoClient::new("194.233.82.254", &8080, "trino", "delta", "ethereum", 0, 10).unwrap();
        let blocks = trino
            .get_blocks::<TrinoEthereumBlock>(10_000_000, 10_000_001)
            .await
            .unwrap();

        assert_eq!(blocks.len(), 200);

        for (idx, block) in blocks.into_iter().enumerate() {
            assert_eq!(10_000_000 + idx, block.get_block_ptr().number as usize);
            let _msg = SerializedDataMessage::from(block);
        }
    }
}
