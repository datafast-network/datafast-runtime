mod ethereum;
mod utils;

use crate::error;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use async_stream::stream;
use ethereum::*;
use prusto::Client;
use prusto::ClientBuilder;
use prusto::Row;
use std::collections::HashSet;
use tokio_stream::Stream;

pub trait TrinoBlockTrait: TryFrom<Row> + Into<SerializedDataMessage> {
    fn get_block_number(&self) -> u64;
    fn get_block_hash(&self) -> String;
    fn get_parent_hash(&self) -> String;
    fn get_insert_timestamp(&self) -> u64;
}

pub struct TrinoClient {
    client: Client,
    start_block: u64,
}

impl TrinoClient {
    pub fn new(
        host: &str,
        port: &u16,
        user: &str,
        catalog: &str,
        schema: &str,
        start_block: u64,
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
    ) -> Result<Vec<R>, SourceError> {
        let query = format!(
            r#"
SELECT * FROM blocks_2
WHERE block_number >= {} AND block_number < {}
"#,
            start_block, stop_block
        );
        let results = self.query(&query).await?;

        let mut blocks = Vec::new();
        let mut block_hashes = HashSet::new();

        for row in results {
            let block = R::try_from(row).map_err(|_| SourceError::TrinoSerializeFail)?;
            let hash = block.get_block_hash();

            if !block_hashes.contains(&hash) {
                blocks.push(block);
                block_hashes.insert(hash);
            }
        }

        blocks.sort_by_key(|b| b.get_block_number());

        Ok(blocks)
    }

    pub async fn get_block_stream<R: TrinoBlockTrait>(
        self,
    ) -> impl Stream<Item = SerializedDataMessage> {
        let mut start_block = self.start_block;
        let step = 100;
        let mut blocks = self
            .get_blocks::<R>(start_block, start_block + step)
            .await
            .unwrap();

        stream! {
            loop {
                for block in blocks {
                    yield Into::<SerializedDataMessage>::into(block)
                }
                start_block += step;
                blocks = self.get_blocks::<R>(start_block, start_block + step).await.unwrap();
            }
        }
    }

    pub async fn get_eth_block_stream(self) -> impl Stream<Item = SerializedDataMessage> {
        self.get_block_stream::<TrinoEthereumBlock>().await
    }
}

#[cfg(test)]
mod test {
    use log::info;

    use super::*;

    #[tokio::test]
    async fn test_trino() {
        env_logger::try_init().unwrap_or_default();

        let trino =
            TrinoClient::new("194.233.82.254", &8080, "trino", "delta", "ethereum", 0).unwrap();
        let blocks = trino
            .get_blocks::<TrinoEthereumBlock>(10_000_000, 10_000_001)
            .await
            .unwrap();

        assert_eq!(blocks.len(), 200);

        for (idx, block) in blocks.into_iter().enumerate() {
            if !block.transactions.is_empty() {
                info!(
                    "block={}, tx length = {:?}",
                    block.get_block_number(),
                    block.transactions.len()
                );
            }
            assert_eq!(10_000_000 + idx, block.get_block_number() as usize);
            let _msg = SerializedDataMessage::from(block);
        }
    }
}
