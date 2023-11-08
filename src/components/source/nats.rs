use crate::config::ContentType;
use crate::errors::SourceError;
use crate::messages::SourceDataMessage;
use async_stream::stream;
use tokio_stream::Stream;

pub struct NatsConsumer {
    conn: nats::Connection,
    subject: String,
    content_type: ContentType,
}

impl NatsConsumer {
    pub fn new(uri: &str, subject: &str, content_type: ContentType) -> Result<Self, SourceError> {
        let conn = nats::connect(uri)?;
        Ok(NatsConsumer {
            conn,
            subject: subject.to_string(),
            content_type,
        })
    }

    pub fn get_subscription_stream(self) -> impl Stream<Item = SourceDataMessage> {
        let sub = self
            .conn
            .subscribe(&self.subject)
            .expect("Failed to subscribe to Nats subject");

        stream! {
            for msg in sub.messages() {
                let serialized_msg = self.serialize_message(&msg).unwrap();
                yield serialized_msg;
                msg.ack().unwrap();
            }
        }
    }

    fn serialize_message(&self, msg: &nats::Message) -> Result<SourceDataMessage, SourceError> {
        match self.content_type {
            ContentType::JSON => {
                let data = serde_json::from_slice(&msg.data)?;
                Ok(SourceDataMessage::JSON(data))
            }
            ContentType::Protobuf => unimplemented!("Protobuf not implemented yet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NatsConsumer;
    use crate::config::ContentType;
    use futures_util::future::join;
    use futures_util::pin_mut;
    use std::fs::File;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_nats() {
        env_logger::try_init().unwrap_or_default();

        let block_data = File::open("./src/tests/blocks/block.json").unwrap();
        let block: serde_json::Value = serde_json::from_reader(block_data).unwrap();
        let block_bytes = serde_json::to_vec(&block).unwrap();
        let publisher = nats::connect("localhost").unwrap();
        let subject = "ethereum";

        let (sender, receive) = kanal::bounded_async::<crate::messages::SourceDataMessage>(1);

        let sub = NatsConsumer::new("nats://localhost:4222", subject, ContentType::JSON).unwrap();

        log::info!("Setup tasks");

        let t1 = async {
            publisher.publish(subject, &block_bytes).unwrap();
        };

        let t2 = async move {
            let s = sub.get_subscription_stream();
            pin_mut!(s);
            while let Some(msg) = s.next().await {
                sender.send(msg).await.unwrap();
            }
        };

        let t3 = async {
            while let Ok(msg) = receive.recv().await {
                log::info!("Received message: {:?}", msg);
                return;
            }
        };

        tokio::select! {
            _ = t3 => {
                log::info!("receive done");
            },
            _ = join(t1, t2) => {
                log::info!("t1 and t3 done");
            }
        }
    }
}
