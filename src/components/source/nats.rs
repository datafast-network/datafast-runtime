use crate::config::ContentType;
use crate::errors::SourceError;
use crate::messages::SourceDataMessage;
use kanal::AsyncSender;

pub struct NatsConsumer {
    conn: nats::Connection,
    subj: String,
    content_type: ContentType,
}

impl NatsConsumer {
    pub fn new(uri: &str, subject: &str, content_type: ContentType) -> Result<Self, SourceError> {
        let conn = nats::connect(uri)?;
        Ok(NatsConsumer {
            conn,
            subj: subject.to_string(),
            content_type,
        })
    }

    fn get_subscription(&self) -> Result<nats::Subscription, SourceError> {
        self.conn
            .subscribe(&self.subj)
            .map_err(SourceError::NatsError)
    }

    fn serialize_message(&self, msg: &nats::Message) -> Result<SourceDataMessage, SourceError> {
        let raw_data = msg.data.clone();
        match self.content_type {
            ContentType::JSON => {
                let data = serde_json::from_slice(&raw_data)?;
                Ok(SourceDataMessage::JSON(data))
            }
            ContentType::Protobuf => unimplemented!("Protobuf not implemented yet"),
        }
    }

    pub async fn consume(&self, sender: AsyncSender<SourceDataMessage>) -> Result<(), SourceError> {
        let sub = self.get_subscription()?;
        log::info!("Subscribed to subject: {}", self.subj);
        while let Some(msg) = sub.next() {
            let source_data = self.serialize_message(&msg)?;
            log::info!("Received message: {:?}", source_data);
            sender.send(source_data).await?;
            msg.ack()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::components::source::nats::NatsConsumer;
    use futures_util::future::join;
    use std::fs::File;

    #[tokio::test]

    async fn test_send_message() {
        env_logger::try_init().unwrap_or_default();
        let block_data = File::open("./src/tests/blocks/block.json").unwrap();
        let block: serde_json::Value = serde_json::from_reader(block_data).unwrap();
        let block_bytes = serde_json::to_vec(&block).unwrap();
        let publisher = nats::connect("localhost").unwrap();
        let subj = "ethereum";
        let (sender, receive) = kanal::bounded_async::<crate::messages::SourceDataMessage>(1);
        let sub = NatsConsumer::new(
            "nats://localhost:4222",
            subj,
            crate::config::ContentType::JSON,
        )
        .unwrap();
        log::info!("Sending message");
        let t1 = async {
            publisher.publish(subj, &block_bytes).unwrap();
        };
        let t3 = sub.consume(sender);
        let t2 = async {
            while let Ok(msg) = receive.recv().await {
                log::info!("Received message: {:?}", msg);
                return;
            }
        };

        tokio::select! {
            _ = t2 => {
                log::info!("receive done");
            },
            _ = join(t1, t3) => {
                log::info!("t1 and t3 done");
            }
        }
    }
}
