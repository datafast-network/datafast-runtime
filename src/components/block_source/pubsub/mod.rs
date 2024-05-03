use crate::common::BlockDataMessage;
use crate::components::block_source::delta::DeltaBlockTrait;
use crate::errors::SourceError;
use crate::proto::ethereum::Block;
use futures_util::StreamExt;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscription::Subscription;
use kanal::AsyncSender;
use prost::Message;

#[derive(Clone)]
pub struct PubSubSource {
    sub: Subscription,
    compression: bool,
}

impl PubSubSource {
    pub async fn new(sub_id: String, compression: bool) -> Result<Self, SourceError> {
        let cfg = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| SourceError::PubSubError(format!("Failed to auth pubsub: {:?}", e)))?;
        let client = Client::new(cfg).await.map_err(|e| {
            SourceError::PubSubError(format!("Failed to create pubsub client: {:?}", e))
        })?;
        let sub = client.subscription(&sub_id);
        Ok(Self { sub, compression })
    }

    pub async fn get_block_stream<R: DeltaBlockTrait>(
        &self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
    ) -> Result<(), SourceError> {
        let mut stream = self.sub.subscribe(None).await?;
        while let Some(message) = stream.next().await {
            let block = if self.compression {
                let block_compressed = lz4::block::decompress(&message.message.data, None)
                    .map_err(|e| {
                        SourceError::DecodeMessageError(format!(
                            "Failed to decompress block: {:?}",
                            e
                        ))
                    })?;
                Block::decode(&block_compressed)
                    .map_err(|e| {
                        SourceError::DecodeMessageError(format!("Failed to decode block: {:?}", e))
                    })
                    .map(BlockDataMessage::from)?
            } else {
                Block::decode(&message.message.data)
                    .map_err(|e| {
                        SourceError::DecodeMessageError(format!("Failed to decode block: {:?}", e))
                    })
                    .map(BlockDataMessage::from)?
            };
            sender
                .send(vec![block])
                .await
                .map_err(|e| SourceError::ChannelSendFail(e))?;
            if let Err(e) = message.ack().await {
                return Err(SourceError::PubSubError(format!(
                    "Failed to ack message: {:?}",
                    e
                )))?;
            }
        }
        Ok(())
    }
}
