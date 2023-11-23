mod ethereum;
mod utils;

use std::collections::HashSet;

use crate::error;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use async_stream::stream;
use ethereum::*;
use prusto::Client;
use prusto::ClientBuilder;
use prusto::Row;
use tokio_stream::Stream;

pub trait TrinoBlockTrait: TryFrom<Row> + Into<SerializedDataMessage> {
    fn get_block_number(&self) -> u64;
    fn get_block_hash(&self) -> String;
    fn get_parent_hash(&self) -> String;
    fn get_insert_timestamp(&self) -> u64;
}

pub struct TrinoClient {
    client: Client,
}

impl TrinoClient {
    pub fn new(
        host: &str,
        port: &u16,
        user: &str,
        catalog: &str,
        schema: &str,
    ) -> Result<Self, SourceError> {
        let client = ClientBuilder::new(user, host)
            .port(port.to_owned())
            .catalog(catalog)
            .schema(schema)
            .build()
            .unwrap();
        Ok(Self { client })
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
    async fn get_blocks<R: TrinoBlockTrait>(&self, query: &str) -> Result<Vec<R>, SourceError> {
        let results = self.query(query).await?;

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
        let mut messages: Vec<SerializedDataMessage> = vec![];

        stream! {
            loop {
                for msg in messages {
                    yield msg
                }
                let blocks = self.get_blocks::<R>("some-query").await.unwrap();
                messages = blocks.into_iter().map(|b| Into::<SerializedDataMessage>::into(b)).collect();
            }
        }
    }

    pub async fn get_eth_block_stream(self) -> impl Stream<Item = SerializedDataMessage> {
        self.get_block_stream::<TrinoEthereumBlock>().await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_trino() {
        env_logger::try_init().unwrap_or_default();

        let trino = TrinoClient::new("localhost", &8080, "trino", "delta", "ethereum").unwrap();
        let blocks = trino
            .get_blocks::<TrinoEthereumBlock>(
                r#"
SELECT * FROM ethereum
WHERE block_number >= 10000000 AND block_number < 10005000
"#,
            )
            .await
            .unwrap();

        assert_eq!(blocks.len(), 5000);

        for (idx, block) in blocks.into_iter().enumerate() {
            assert_eq!(10_000_000 + idx, block.get_block_number() as usize);
            let _msg = SerializedDataMessage::from(block);
        }
    }
}
