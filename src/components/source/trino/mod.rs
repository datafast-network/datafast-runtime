use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use async_stream::stream;
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

    async fn query(&self, query: String) -> Result<Vec<Row>, SourceError> {
        Ok(self
            .client
            .get_all::<Row>(query)
            .await
            .map_err(|_| SourceError::TrinoQueryFail)?
            .into_vec())
    }

    async fn get_blocks<R: TryFrom<Row> + Into<SerializedDataMessage>>(
        &self,
        query: String,
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
                let blocks = self.get_blocks::<R>("some-query".to_string()).await.unwrap();
                messages = blocks.into_iter().map(|b| Into::<SerializedDataMessage>::into(b)).collect();
            }
        }
    }
}

#[cfg(test)]
mod test {}
