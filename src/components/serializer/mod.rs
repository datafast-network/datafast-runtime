use crate::config::Config;
use crate::errors::SerializerError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use kanal::AsyncReceiver;
use kanal::AsyncSender;
use prometheus::Registry;

pub enum Serializer {
    DirectSerializer,
}

impl Serializer {
    pub fn new(_config: &Config, _registry: &Registry) -> Result<Self, SerializerError> {
        Ok(Self::DirectSerializer)
    }

    pub async fn run_async(
        self,
        source_recv: AsyncReceiver<SourceDataMessage>,
        result_sender: AsyncSender<SerializedDataMessage>,
    ) -> Result<(), SerializerError> {
        match self {
            Self::DirectSerializer => {
                while let Ok(source) = source_recv.recv().await {
                    match source {
                        SourceDataMessage::AlreadySerialized(msg) => {
                            result_sender.send(msg).await?
                        }
                        _ => unimplemented!(),
                    }
                }
            }
        };

        Ok(())
    }
}
