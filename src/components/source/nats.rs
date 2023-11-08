use crate::errors::SourceErr;
use crate::messages::SourceDataMessage;
use kanal::AsyncSender;

pub struct NatsConsumer {
    conn: nats::Connection,
    subj: String,
}

impl NatsConsumer {
    pub fn new(uri: &str, subject: &str) -> Result<Self, SourceErr> {
        let conn = nats::connect(uri)?;
        Ok(NatsConsumer {
            conn,
            subj: subject.to_string(),
        })
    }

    pub async fn run_async(self, sender: AsyncSender<SourceDataMessage>) -> Result<(), SourceErr> {
        let sub = self.conn.subscribe(&self.subj)?;
        while let Some(msg) = sub.next() {
            let value: serde_json::Value = serde_json::from_slice(&msg.data)?;
            sender.send(SourceDataMessage::JSON(value)).await?;
        }
        Ok(())
    }
}
