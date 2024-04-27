use crate::common::BlockDataMessage;
use crate::errors::SourceError;
use crate::proto::ethereum::Block;
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
    sub_id: String,
}

impl PubSubSource {
    pub async fn new(topic: String, sub_id: String) -> Result<Self, SourceError> {
        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| SourceError::PubSubAuthError(e.to_string()))?;

        let client = Client::new(config).await?;
        Ok(Self {
            client,
            topic,
            sub_id,
        })
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
        if !subscription
            .exists(None)
            .await
            .map_err(|_| SourceError::PubSubAuthError("create subscription error".to_string()))?
        {
            subscription
                .create(topic.fully_qualified_name(), sub_cfg, None)
                .await
                .map_err(|_| {
                    SourceError::PubSubAuthError("create subscription error".to_string())
                })?;
        }
        let mut stream = subscription
            .subscribe(None)
            .await
            .map_err(|_| SourceError::PubSubAuthError("create subscription error".to_string()))?;
        while let Some(message) = stream.next().await {
            let block = {
                if cfg!(feature = "pubsub_compress") {
                    let compress_data =
                        lz4::block::decompress(message.message.data.as_slice(), None)
                            .map_err(|e| SourceError::PubSubDecodeError(e.to_string()))?;

                    Block::decode(compress_data.as_slice())
                        .map(BlockDataMessage::from)
                        .map_err(|e| SourceError::PubSubDecodeError(e.to_string()))?
                } else {
                    Block::decode(message.message.data.as_slice())
                        .map(BlockDataMessage::from)
                        .map_err(|e| SourceError::PubSubDecodeError(e.to_string()))?
                }
            };

            if sender.send(vec![block]).await.is_ok() {
                message
                    .ack()
                    .await
                    .map_err(|_| SourceError::PubSubDecodeError("Ack message error".to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::ethereum::Block;
    use std::str::FromStr;
    use web3::types::H256;

    #[test]
    fn test_decompress_block() {
        let block_bytes = vec![
            8, 1, 18, 66, 48, 120, 56, 56, 101, 57, 54, 100, 52, 53, 51, 55, 98, 101, 97, 52, 100,
            57, 99, 48, 53, 100, 49, 50, 53, 52, 57, 57, 48, 55, 98, 51, 50, 53, 54, 49, 100, 51,
            98, 102, 51, 49, 102, 52, 53, 97, 97, 101, 55, 51, 52, 99, 100, 99, 49, 49, 57, 102,
            49, 51, 52, 48, 54, 99, 98, 54, 26, 66, 48, 120, 100, 52, 101, 53, 54, 55, 52, 48, 102,
            56, 55, 54, 97, 101, 102, 56, 99, 48, 49, 48, 98, 56, 54, 97, 52, 48, 100, 53, 102, 53,
            54, 55, 52, 53, 97, 49, 49, 56, 100, 48, 57, 48, 54, 97, 51, 52, 101, 54, 57, 97, 101,
            99, 56, 99, 48, 100, 98, 49, 99, 98, 56, 102, 97, 51, 32, 1, 42, 249, 6, 10, 42, 48,
            120, 48, 53, 97, 53, 54, 101, 50, 100, 53, 50, 99, 56, 49, 55, 49, 54, 49, 56, 56, 51,
            102, 53, 48, 99, 52, 52, 49, 99, 51, 50, 50, 56, 99, 102, 101, 53, 52, 100, 57, 102,
            18, 66, 48, 120, 100, 54, 55, 101, 52, 100, 52, 53, 48, 51, 52, 51, 48, 52, 54, 52, 50,
            53, 97, 101, 52, 50, 55, 49, 52, 55, 52, 51, 53, 51, 56, 53, 55, 97, 98, 56, 54, 48,
            100, 98, 99, 48, 97, 49, 100, 100, 101, 54, 52, 98, 52, 49, 98, 53, 99, 100, 51, 97,
            53, 51, 50, 98, 102, 51, 26, 66, 48, 120, 53, 54, 101, 56, 49, 102, 49, 55, 49, 98, 99,
            99, 53, 53, 97, 54, 102, 102, 56, 51, 52, 53, 101, 54, 57, 50, 99, 48, 102, 56, 54,
            101, 53, 98, 52, 56, 101, 48, 49, 98, 57, 57, 54, 99, 97, 100, 99, 48, 48, 49, 54, 50,
            50, 102, 98, 53, 101, 51, 54, 51, 98, 52, 50, 49, 34, 66, 48, 120, 53, 54, 101, 56, 49,
            102, 49, 55, 49, 98, 99, 99, 53, 53, 97, 54, 102, 102, 56, 51, 52, 53, 101, 54, 57, 50,
            99, 48, 102, 56, 54, 101, 53, 98, 52, 56, 101, 48, 49, 98, 57, 57, 54, 99, 97, 100, 99,
            48, 48, 49, 54, 50, 50, 102, 98, 53, 101, 51, 54, 51, 98, 52, 50, 49, 42, 1, 48, 50, 4,
            53, 48, 48, 48, 58, 52, 48, 120, 52, 55, 54, 53, 55, 52, 54, 56, 50, 102, 55, 54, 51,
            49, 50, 101, 51, 48, 50, 101, 51, 48, 50, 102, 54, 99, 54, 57, 54, 101, 55, 53, 55, 56,
            50, 102, 54, 55, 54, 102, 51, 49, 50, 101, 51, 52, 50, 101, 51, 50, 66, 130, 4, 48,
            120, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 74, 10, 49, 52, 51, 56, 50, 54, 57, 57, 56, 56, 82, 11,
            49, 55, 49, 55, 49, 52, 56, 48, 53, 55, 54, 90, 11, 51, 52, 51, 53, 49, 51, 52, 57, 55,
            54, 48, 104, 153, 4, 122, 18, 48, 120, 53, 51, 57, 98, 100, 52, 57, 55, 57, 102, 101,
            102, 49, 101, 99, 52,
        ];
        let block_compress = lz4::block::compress(block_bytes.as_slice(), None, true).unwrap();
        let block_decompress = lz4::block::decompress(block_compress.as_slice(), None).unwrap();
        let block = Block::decode(block_decompress.as_slice()).unwrap();
        assert_eq!(block.block_number, 1);
        assert_eq!(block.chain_id, 1);
        let expected_hash =
            H256::from_str("0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6")
                .unwrap();
        let expected_parent_hash =
            H256::from_str("0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3")
                .unwrap();
        assert_eq!(block.block_hash, format!("{:?}", expected_hash));
        assert_eq!(block.parent_hash, format!("{:?}", expected_parent_hash));
    }
}
