mod transform;

use super::database::Agent;
use crate::config::Config;
use crate::debug;
use crate::errors::SerializerError;
use crate::errors::TransformError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use crate::runtime::wasm_host::create_wasm_host;
use kanal::AsyncReceiver;
use kanal::AsyncSender;
use prometheus::Registry;
use semver::Version;
use transform::Transform;

pub enum Serializer {
    Transform(Transform),
    #[allow(dead_code)]
    DirectSerializer,
}

impl Serializer {
    pub fn new(config: Config, registry: &Registry) -> Result<Self, SerializerError> {
        match config.transform {
            Some(transform_cfg) => {
                if config.transform_wasm.is_none() {
                    return Err(SerializerError::TransformError(
                        TransformError::MissingTransformWASM,
                    ));
                }

                let transform_wasm = config.transform_wasm.clone().unwrap();
                let wasm_bytes = std::fs::read(config.transform_wasm.unwrap())
                    .map_err(|_| TransformError::BadTransformWasm(transform_wasm))?;
                let empty_db = Agent::empty(registry);
                let wasm_version = Version::new(0, 0, 5);
                let host =
                    create_wasm_host(wasm_version, wasm_bytes, empty_db, "Serializer".to_string())?;
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
        source_recv: AsyncReceiver<SourceDataMessage>,
        result_sender: AsyncSender<SerializedDataMessage>,
    ) -> Result<(), SerializerError> {
        match self {
            Self::Transform(mut transform) => {
                while let Ok(source) = source_recv.recv().await {
                    debug!(Transform, "Received source data");
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
