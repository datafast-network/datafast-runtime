use crate::config::ContentType;
use crate::debug;
use crate::errors::SourceError;
use crate::info;
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

    pub fn get_subscription_stream(self) -> impl Stream<Item = Vec<u8>> {
        let sub = self
            .conn
            .subscribe(&self.subject)
            .expect("Failed to subscribe to Nats subject");

        stream! {
            for msg in sub.messages() {
                yield msg.data.clone();
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
}

#[cfg(test)]
mod tests {

    use crate::proto::ethereum::Block;
    use prost::Message;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_decode_protobuf_message() {
        let mut proto_content = File::open("./src/tests/blocks/block.bin").unwrap();
        let mut buffer = vec![];
        proto_content.read_to_end(&mut buffer).unwrap();
        let block = Block::decode(buffer.as_slice()).unwrap();
        assert_eq!(block.block_number, 10000000);
    }
}
