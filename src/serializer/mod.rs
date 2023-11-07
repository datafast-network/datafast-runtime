mod transform;

use kanal::AsyncReceiver;
use kanal::AsyncSender;
use semver::Version;
use transform::Transform;

use crate::config::Config;
use crate::database::DatabaseAgent;
use crate::errors::SerializerError;
use crate::messages::SourceInputMessage;
use crate::messages::TransformedDataMessage;
use crate::wasm_host::create_wasm_host;

pub enum Serializer {
    Transform(Transform),
    DirectSerializer,
}

impl Serializer {
    pub fn new(config: Config) -> Result<Self, SerializerError> {
        match config.transform {
            Some(transform_cfg) => {
                let empty_db = DatabaseAgent::default();
                let wasm_version = Version::new(0, 0, 5);
                let host = create_wasm_host(wasm_version, vec![], empty_db)?;
                let transform = Transform::new(host, config.chain, transform_cfg)?;
                Ok(Self::Transform(transform))
            }
            _ => {
                todo!("Implement raw data serialization into real struct")
            }
        }
    }

    pub async fn run_async(
        self,
        source_recv: AsyncReceiver<SourceInputMessage>,
        result_sender: AsyncSender<TransformedDataMessage>,
    ) -> Result<(), SerializerError> {
        match self {
            Self::Transform(mut transform) => {
                while let Ok(source) = source_recv.recv().await {
                    result_sender
                        .send(transform.handle_source_input(source)?)
                        .await?
                }
            }

            Self::DirectSerializer => {
                todo!("implement raw data serialization")
            }
        };

        Ok(())
    }
}
