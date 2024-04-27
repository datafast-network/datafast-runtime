use crate::common::proto::ethereum::Block;
use crate::common::BlockDataMessage;
use crate::errors::SourceError;
use futures_util::StreamExt;
use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::client::ClientConfig;
use google_cloud_pubsub::subscription::SubscriptionConfig;
use kanal::AsyncSender;
use prost::Message;

#[derive(Clone)]
pub struct PubSubSource {
    client: Client,
    topic: String,
    sub_id: String
}

impl PubSubSource {
    pub async fn new(topic: String, sub_id: String) -> Result<(Self), SourceError> {
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;
        Ok(Self { client, topic, sub_id })
    }

    pub async fn subscribe(
        &self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
    ) -> Result<(), SourceError> {
        let topic = self.client.topic(&self.topic);
        let sub_cfg = SubscriptionConfig {
            enable_message_ordering: true,
            ..Default::default()
        };

        let subscription = self.client.subscription(&self.sub_id);
        if !subscription.exists(None).await? {
            subscription
                .create(topic.fully_qualified_name(), sub_cfg, None)
                .await?;
        }
        let mut stream = subscription.subscribe(None).await?;
        while let Some(message) = stream.next().await {
            let block = Block::decode(message.message.data.as_slice())
                .map(BlockDataMessage::from)
                .map_err(|e| SourceError::PubSubDecodeError(e.to_string()))?;

            if sender.send(vec![block]).await.is_ok() {
                message.ack().await?
            }
        }

        Ok(())
    }
}
