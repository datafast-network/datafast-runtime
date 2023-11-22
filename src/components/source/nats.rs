use crate::config::ContentType;
use crate::debug;
use crate::errors::SourceError;
use crate::info;
use crate::messages::SourceDataMessage;
use crate::proto::ethereum::Block;
use async_stream::stream;
use prost::Message;
use std::fs::File;
use std::io::Write;
use tokio_stream::Stream;

pub struct NatsConsumer {
    conn: nats::Connection,
    subject: String,
    content_type: ContentType,
}

impl NatsConsumer {
    pub fn new(uri: &str, subject: &str, content_type: ContentType) -> Result<Self, SourceError> {
        let conn = nats::connect(uri)?;
        info!(NatsConsumer,"Connected to Nats";
            uri => uri,
            subject => subject,
            content_type => format!("{:?}", content_type)
        );
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
                debug!(NatsConsumer, "Received data");
                yield serialized_msg;
                if let Err(e) = msg.ack(){
                    debug!(
                        NatsConsumer,
                        "Failed to ack message";
                        error => e.to_string()
                    );
                }else{
                    debug!(NatsConsumer, "Acked message");
                }
            }
        }
    }

    fn serialize_message(&self, msg: &nats::Message) -> Result<SourceDataMessage, SourceError> {
        match self.content_type {
            ContentType::Json => {
                let data = serde_json::from_slice(&msg.data)?;
                Ok(SourceDataMessage::Json(data))
            }
            ContentType::Protobuf => {
                let start = tokio::time::Instant::now();
                let block = Block::decode(msg.data.as_slice()).unwrap();
                let full_block = block.try_into().unwrap();
                info!(
                    NatsConsumer,
                    "Decoded block";
                    time => format!("{:?}", start.elapsed())
                );
                Ok(SourceDataMessage::Protobuf(vec![full_block]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NatsConsumer;
    use crate::config::ContentType;
    use crate::proto::ethereum::Block;
    use futures_util::future::join;
    use futures_util::pin_mut;
    use google_cloud_googleapis::pubsub::v1::PubsubMessage;
    use prost::Message;
    use std::fs::File;
    use std::io::Read;
    use tokio_stream::StreamExt;

    #[test]
    fn test_decode_protobuf_message() {
        let mut proto_content = File::open("./src/tests/blocks/block.bin").unwrap();
        let mut buffer = vec![];
        proto_content.read_to_end(&mut buffer).unwrap();
        let block = Block::decode(buffer.as_slice()).unwrap();
        assert_eq!(block.block_number, 10000000);
    }

    #[tokio::test]
    async fn test_nats() {
        env_logger::try_init().unwrap_or_default();

        // let block_data = File::open("./src/tests/blocks/block.json").unwrap();
        // let block: serde_json::Value = serde_json::from_reader(block_data).unwrap();
        // let block_bytes = serde_json::to_vec(&block).unwrap();
        // let publisher = nats::connect("localhost").unwrap();
        // let subject = "ethereum";
        //
        // let (sender, receive) = kanal::bounded_async::<crate::messages::SourceDataMessage>(1);
        //
        // let sub = NatsConsumer::new("nats://localhost:4222", subject, ContentType::Json).unwrap();
        //
        // log::info!("Setup tasks");
        //
        // let t1 = async {
        //     publisher.publish(subject, &block_bytes).unwrap();
        // };
        //
        // let t2 = async move {
        //     let s = sub.get_subscription_stream();
        //     pin_mut!(s);
        //     while let Some(msg) = s.next().await {
        //         sender.send(msg).await.unwrap();
        //     }
        // };
        //
        // let t3 = async {
        //     if let Ok(msg) = receive.recv().await {
        //         log::info!("Received message: {:?}", msg);
        //     }
        // };
        //
        // tokio::select! {
        //     _ = t3 => {
        //         log::info!("receive done");
        //     },
        //     _ = join(t1, t2) => {
        //         log::info!("t1 and t3 done");
        //     }
        // }
    }
}
