use super::ExternDBTrait;
use crate::errors::DatabaseError;
use mongodb::options::ClientOptions;
use mongodb::Client;

pub struct MongoDB {}

impl MongoDB {
    pub async fn new() -> Result<Self, DatabaseError> {
        let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
        client_options.app_name = Some("My App".to_string());
        Ok(MongoDB {})
    }
}
