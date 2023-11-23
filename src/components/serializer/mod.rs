use crate::config::Config;
use crate::errors::SerializerError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use kanal::AsyncReceiver;
use kanal::AsyncSender;
use prometheus::Registry;

pub enum Serializer {
    Protobuf,
}

impl Serializer {
    pub fn new(_config: &Config, _registry: &Registry) -> Result<Self, SerializerError> {
        Ok(Self::Protobuf)
    }

    pub async fn run_async(
        self,
        source_recv: AsyncReceiver<SourceDataMessage>,
        _result_sender: AsyncSender<SerializedDataMessage>,
    ) -> Result<(), SerializerError> {
        while let Ok(source) = source_recv.recv().await {
            match source {
                SourceDataMessage::Protobuf(_data) => {
                    todo!()
                }
                SourceDataMessage::Json(_data) => {
                    todo!()
                }
            }
        }
        Ok(())
    }
}
