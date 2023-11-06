use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::chain::ethereum::block::AscEthereumBlock;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::log::AscLogArray;
use crate::chain::ethereum::transaction::AscTransactionArray;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::Chain;
use crate::config::Config;
use crate::config::TransformConfig;
use crate::errors::TransformError;
use crate::messages::SourceInputMessage;
use crate::messages::TransformedDataMessage;
use crate::wasm_host::AscHost;
use kanal::AsyncReceiver;
use kanal::AsyncSender;
use std::collections::HashMap;
use wasmer::Function;
use wasmer::RuntimeError;
use wasmer::Value;
use web3::types::Log;

pub struct Transform {
    host: AscHost,
    funcs: HashMap<String, Function>,
    config: Option<TransformConfig>,
    chain: Chain,
}

impl Transform {
    pub fn new(host: AscHost, conf: &Config) -> Self {
        Transform {
            host,
            funcs: HashMap::new(),
            config: conf.transforms.clone(),
            chain: conf.chain.clone(),
        }
    }

    pub fn bind_transform_functions(mut self) -> Result<Self, TransformError> {
        if self.config.is_none() {
            return Ok(self);
        }
        let conf = self.config.unwrap();
        match conf {
            TransformConfig::Ethereum {
                block,
                transactions,
                logs,
            } => {
                let block_transform_fn = self.host.instance.exports.get_function(&block)?;
                let txs_transform_fn = self.host.instance.exports.get_function(&transactions)?;
                let logs_transform_fn = self.host.instance.exports.get_function(&logs)?;
                self.funcs.insert(block, block_transform_fn.to_owned());
                self.funcs.insert(transactions, txs_transform_fn.to_owned());
                self.funcs.insert(logs, logs_transform_fn.to_owned())
            }
            _ => unimplemented!(),
        };

        Ok(self)
    }

    fn generic_transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        source: SourceInputMessage,
        function_name: &str,
    ) -> Result<R, TransformError> {
        let func = self
            .funcs
            .get(function_name)
            .ok_or(TransformError::InvalidFunctionName(
                function_name.to_string(),
            ))?;

        let asc_ptr = match source {
            SourceInputMessage::JSON(json_data) => {
                let asc_json = asc_new(&mut self.host, &json_data)?;
                asc_json.wasm_ptr()
            }
            SourceInputMessage::Protobuf => {
                unimplemented!()
            }
        };
        let result = func.call(&mut self.host.store, &[Value::I32(asc_ptr as i32)])?;
        let result_ptr = result
            .first()
            .ok_or(TransformError::TransformFail(RuntimeError::new(
                "Invalid pointer",
            )))?
            .unwrap_i32() as u32;
        let asc_ptr = AscPtr::<P>::new(result_ptr);
        let result = asc_get(&self.host, asc_ptr, 0)?;
        Ok(result)
    }

    fn handle_source_input(
        &mut self,
        source: SourceInputMessage,
    ) -> Result<TransformedDataMessage, TransformError> {
        match (self.chain.clone(), self.config.clone()) {
            (
                Chain::Ethereum,
                Some(TransformConfig::Ethereum {
                    block,
                    transactions,
                    logs,
                }),
            ) => {
                let block = self.generic_transform_data::<AscEthereumBlock, EthereumBlockData>(
                    source.clone(),
                    &block,
                )?;
                let transactions = self
                    .generic_transform_data::<AscTransactionArray, Vec<EthereumTransactionData>>(
                        source.clone(),
                        &transactions,
                    )?;
                let logs = self.generic_transform_data::<AscLogArray, Vec<Log>>(source, &logs)?;
                Ok(TransformedDataMessage::Ethereum {
                    block,
                    transactions,
                    logs,
                })
            }
            (Chain::Ethereum, None) => {
                //TODO: Handle no transforms
                todo!("serialize to transform message")
            }
            _ => Err(TransformError::InvalidChain),
        }
    }

    pub async fn run(
        mut self,
        receiver: AsyncReceiver<SourceInputMessage>,
        sender: AsyncSender<TransformedDataMessage>,
    ) -> Result<(), TransformError> {
        while let Ok(msg) = receiver.recv().await {
            let result = self.handle_source_input(msg)?;
            sender.send(result).await?
        }
        Ok(())
    }
}

//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::wasm_host::test::get_subgraph_testing_resource;
//     use crate::wasm_host::test::mock_wasm_host;
//     use std::fs::File;
//
//     pub fn mock_transform(conf: &Config) -> TransformInstance {
//         let transforms = conf.transforms.as_ref().unwrap().iter().fold(
//             HashMap::new(),
//             |mut trans, (name, trans_conf)| {
//                 let (version, wasm_path) =
//                     get_subgraph_testing_resource("0.0.5", &trans_conf.datasource);
//                 let host = mock_wasm_host(version, &wasm_path);
//                 trans.insert(trans_conf.func_name.clone(), Transform::new(host, conf));
//                 trans
//             },
//         );
//         let (sender, _) = kanal::bounded_async(1);
//         TransformInstance { transforms, sender }
//     }
//
//     #[tokio::test]
//     async fn test_transform_full_block() {
//         env_logger::try_init().unwrap_or_default();
//         let mut transforms = HashMap::new();
//         let transform_block = TransformConfig {
//             datasource: "Ingestor".to_string(),
//             func_name: "transformEthereumBlock".to_string(),
//             wasm_path: "test".to_string(),
//         };
//         transforms.insert(transform_block.func_name.clone(), transform_block.clone());
//         let conf = Config {
//             subgraph_name: "".to_string(),
//             subgraph_id: None,
//             manifest: "".to_string(),
//             transforms: Some(transforms),
//         };
//         let mut transform = mock_transform(&conf);
//         let file_json = File::open("./tests/block.json").unwrap();
//         // Send test data for transform
//         let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
//         let request = TransformRequest {
//             value: ingestor_block.clone(),
//             transform: transform_block,
//         };
//         let data = transform.transform_block(request).unwrap();
//         let block = match data {
//             TransformedDataMessage::Block(block) => block,
//             _ => panic!("Invalid data type"),
//         };
//         assert_eq!(format!("{:?}", block.number), "10000000");
//         //asert_eq all fields of block
//     }
//
//     #[tokio::test]
//     async fn test_transform_txs() {
//         env_logger::try_init().unwrap_or_default();
//         let mut transforms = HashMap::new();
//         let transform_block = TransformConfig {
//             datasource: "Ingestor".to_string(),
//             func_name: "transformEthereumTxs".to_string(),
//             wasm_path: "test".to_string(),
//         };
//         transforms.insert(transform_block.func_name.clone(), transform_block.clone());
//         let conf = Config {
//             subgraph_name: "".to_string(),
//             subgraph_id: None,
//             manifest: "".to_string(),
//             transforms: Some(transforms),
//         };
//         let mut transform = mock_transform(&conf);
//         let file_json = File::open("./src/tests/block.json").unwrap();
//         // Send test data for transform
//         let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
//         let request = TransformRequest {
//             value: ingestor_block.get("transactions").unwrap().clone(),
//             transform: transform_block,
//         };
//         let data = transform.transform_txs(request).unwrap();
//         let txs = match data {
//             TransformedDataMessage::Transactions(txs) => txs,
//             _ => panic!("Invalid data type"),
//         };
//         assert_eq!(txs.len(), 2);
//         // assert_eq!(format!("{:?}", block.len()), "10000000");
//         //asert_eq all fields of block
//     }
// }
