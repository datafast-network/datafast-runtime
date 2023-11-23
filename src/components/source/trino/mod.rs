use crate::config::Config;
use crate::errors::SourceError;
use prusto_rs::Client;
use prusto_rs::ClientBuilder;
use prusto_rs::Row;

pub struct TrinoClient {
    client: Client,
}

impl TrinoClient {
    pub fn new(cfg: &Config) -> Result<Self, SourceError> {
        let source = cfg.source;
        match source {
            crate::config::SourceTypes::Trino {
                host,
                port,
                user,
                catalog,
                schema,
            } => {
                let client = ClientBuilder::new(user, host)
                    .port(port)
                    .catalog(catalog)
                    .schema(schema)
                    .build()
                    .unwrap();
                Ok(Self { client })
            }
            _ => unimplemented!(),
        }
    }

    async fn query(&self, query: String) -> Result<Vec<Row>, SourceError> {
        Ok(self.client.get_all::<Row>(query).await.unwrap().into_vec())
    }

    async fn get_blocks<R: TryFrom<Row>>(&self, query: String) -> Result<Vec<R>, SourceError> {
        let results = self.query(query).await?;
        let mut blocks = Vec::new();
        for row in results {
            let block = R::try_from(row).map_err(|_| SourceError::TrinoSerializeFail)?;
            blocks.push(block);
        }
        Ok(blocks)
    }
}
