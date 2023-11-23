mod ethereum;

use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use async_stream::stream;
use ethereum::*;
use prusto_rs::Client;
use prusto_rs::ClientBuilder;
use prusto_rs::Row;
use tokio_stream::Stream;

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
            .map_err(|_| SourceError::TrinoQueryFail)?
            .into_vec())
    }

    async fn get_blocks<R: TryFrom<Row> + Into<SerializedDataMessage>>(
        &self,
        query: &str,
    ) -> Result<Vec<R>, SourceError> {
        let results = self.query(query).await?;
        let mut blocks = Vec::new();
        for row in results {
            let block = R::try_from(row).map_err(|_| SourceError::TrinoSerializeFail)?;
            blocks.push(block);
        }
        Ok(blocks)
    }

    pub async fn get_block_stream<R: TryFrom<Row> + Into<SerializedDataMessage>>(
        self,
    ) -> impl Stream<Item = SourceDataMessage> {
        let mut messages: Vec<SerializedDataMessage> = vec![];

        stream! {
            loop {
                for msg in messages {
                    yield SourceDataMessage::AlreadySerialized(msg);
                }
                let blocks = self.get_blocks::<R>("some-query").await.unwrap();
                messages = blocks.into_iter().map(|b| Into::<SerializedDataMessage>::into(b)).collect();
            }
        }
    }

    pub async fn get_eth_block_stream(self) -> impl Stream<Item = SourceDataMessage> {
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

        let trino = TrinoClient::new("localhost", &8080, "trino", "delta", "mockchain").unwrap();
        let rows = trino
            .query(
                "select * from blocks where block_number > 5000 and block_number < 10000 limit 1",
            )
            .await
            .unwrap();

        for row in rows {
            info!("rows {:?}", row.into_json());
        }
    }
}
