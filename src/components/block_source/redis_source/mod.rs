use crate::common::BlockDataMessage;
use crate::components::block_source::proto::ethereum::Block as PbBlock;
use crate::errors::SourceError;
use kanal::AsyncSender;
use log::info;
use prost::Message;
use tokio::sync::RwLock;

pub struct RedisSource {
    redis_channel: String,
    client: RwLock<redis::Connection>,
}

impl RedisSource {
    pub async fn new(redis_uri: &str, redis_channel: &str) -> Result<Self, SourceError> {
        let client = redis::Client::open(redis_uri)
            .map_err(|e| SourceError::Initialization(e.to_string()))?
            .get_connection()
            .map_err(|e| SourceError::Initialization(e.to_string()))?;
        info!("Connected to redis: {}", redis_uri);
        Ok(Self {
            redis_channel: redis_channel.to_string(),
            client: RwLock::new(client),
        })
    }

    pub async fn subscribe(
        &self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
    ) -> Result<(), SourceError> {
        let mut conn = self.client.write().await;
        let mut sub = conn.as_pubsub();
        sub.subscribe(&self.redis_channel)
            .map_err(|e| SourceError::Initialization(e.to_string()))?;
        loop {
            match sub.get_message() {
                Ok(msg) => {
                    let payload: Vec<u8> = msg.get_payload().unwrap();
                    let block = PbBlock::decode(payload.as_slice())
                        .map_err(|e| SourceError::ParseRedisMessage(e.to_string()))
                        .map(|b| BlockDataMessage::from(b))?;
                    sender.send(vec![block]).await?;
                }
                Err(e) => {
                    log::error!("Error getting message: {:?}", e);
                }
            }
        }
    }
}
