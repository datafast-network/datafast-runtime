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
