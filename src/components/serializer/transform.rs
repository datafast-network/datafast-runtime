use crate::chain::ethereum::block::AscEthereumBlock;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::log::AscLogArray;
use crate::chain::ethereum::transaction::AscTransactionArray;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::Chain;
use crate::config::TransformConfig;
use crate::errors::TransformError;
use crate::log_info;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::FromAscObj;
use crate::runtime::wasm_host::AscHost;
use std::collections::HashMap;
use wasmer::Function;
use wasmer::Value;
use web3::types::Log;

pub struct Transform {
    host: AscHost,
    funcs: HashMap<String, Function>,
    config: TransformConfig,
    chain: Chain,
}

impl Transform {
    pub fn new(
        host: AscHost,
        chain: Chain,
        config: TransformConfig,
    ) -> Result<Self, TransformError> {
        let this = Transform {
            host,
            funcs: HashMap::new(),
            config,
            chain,
        };
        this.bind_transform_functions()
    }

    pub fn bind_transform_functions(mut self) -> Result<Self, TransformError> {
        match self.config.clone() {
            TransformConfig::Ethereum {
                block,
                transactions,
                logs,
            } => {
                log_info!(Transform, "Transform initialized";
                    "chain" => format!("{:?}", self.chain),
                    "block" => block,
                    "transactions" => transactions,
                    "logs" => logs);

                let exports = &self.host.instance.exports;
                let block_transform_fn = exports
                    .get_function(&block)
                    .map_err(|_| TransformError::InvalidFunctionName(block.to_owned()))?
                    .to_owned();
                let txs_transform_fn = exports
                    .get_function(&transactions)
                    .map_err(|_| TransformError::InvalidFunctionName(transactions.to_owned()))?
                    .to_owned();
                let logs_transform_fn = exports
                    .get_function(&logs)
                    .map_err(|_| TransformError::InvalidFunctionName(logs.to_owned()))?
                    .to_owned();
                self.funcs.insert(block, block_transform_fn);
                self.funcs.insert(transactions, txs_transform_fn.to_owned());
                self.funcs.insert(logs, logs_transform_fn.to_owned());
            }
            _ => unimplemented!(),
        };

        Ok(self)
    }

    fn generic_transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        source: SourceDataMessage,
        function_name: &str,
    ) -> Result<R, TransformError> {
        let func = self
            .funcs
            .get(function_name)
            .ok_or(TransformError::InvalidFunctionName(
                function_name.to_string(),
            ))?;

        let asc_ptr = match source {
            SourceDataMessage::JSON(json_data) => {
                let asc_json = asc_new(&mut self.host, &json_data)?;
                asc_json.wasm_ptr() as i32
            }
            SourceDataMessage::Protobuf => {
                unimplemented!()
            }
        };
        let result = func.call(&mut self.host.store, &[Value::I32(asc_ptr)])?;
        let result_ptr = result
            .first()
            .ok_or(TransformError::TransformReturnNoValue)?
            .unwrap_i32() as u32;
        let asc_ptr = AscPtr::<P>::new(result_ptr);
        let result = asc_get(&self.host, asc_ptr, 0)?;
        Ok(result)
    }

    pub fn handle_source_input(
        &mut self,
        source: SourceDataMessage,
    ) -> Result<SerializedDataMessage, TransformError> {
        match (self.chain.clone(), self.config.clone()) {
            (
                Chain::Ethereum,
                TransformConfig::Ethereum {
                    block,
                    transactions,
                    logs,
                },
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
                Ok(SerializedDataMessage::Ethereum {
                    block,
                    transactions,
                    logs,
                })
            }
            _ => Err(TransformError::ChainMismatched),
        }
    }
}
