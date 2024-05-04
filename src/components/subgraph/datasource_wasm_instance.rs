use crate::common::Datasource;
use crate::common::DatasourceBundle;
use crate::common::HandlerTypes;
use crate::components::ManifestAgent;
use crate::database::DatabaseAgent;
use crate::errors::SubgraphError;
use crate::rpc_client::RpcAgent;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::ToAscObj;
use crate::runtime::wasm_host::AscHost;
use std::collections::HashMap;
use wasmer::Exports;
use wasmer::Function;
use wasmer::Value;

pub struct Handler {
    pub name: String,
    inner: Function,
}

impl Handler {
    pub fn new(instance_exports: &Exports, func_name: &str) -> Result<Self, SubgraphError> {
        let this = Self {
            name: func_name.to_string(),
            inner: instance_exports
                .get_function(func_name)
                .map_err(|_| SubgraphError::InvalidHandlerName(func_name.to_owned()))?
                .to_owned(),
        };
        Ok(this)
    }
}

pub struct EthereumHandlers {
    pub block: HashMap<String, Handler>,
    pub transaction: HashMap<String, Handler>,
    pub events: HashMap<String, Handler>,
}

pub struct DatasourceWasmInstance {
    pub name: String,
    // NOTE: Add more chain-based handler here....
    pub ethereum_handlers: EthereumHandlers,
    host: AscHost,
}

impl TryFrom<(&AscHost, &Datasource)> for EthereumHandlers {
    type Error = SubgraphError;
    fn try_from((host, ds): (&AscHost, &Datasource)) -> Result<Self, SubgraphError> {
        let mut eth_event_handlers = HashMap::new();
        let mut eth_block_handlers = HashMap::new();
        let mut eth_transaction_handlers = HashMap::new();

        for event_handler in ds.mapping.eventHandlers.clone().unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-event handler, must fix later
            let handler = Handler::new(&host.instance.exports, &event_handler.handler)?;
            eth_event_handlers.insert(event_handler.handler.to_owned(), handler);
        }

        for block_handler in ds.mapping.blockHandlers.clone().unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-block handler, must fix later
            let handler = Handler::new(&host.instance.exports, &block_handler.handler)?;
            eth_block_handlers.insert(block_handler.handler.to_owned(), handler);
        }

        for transaction_handler in ds
            .mapping
            .transactionHandlers
            .clone()
            .unwrap_or_default()
            .iter()
        {
            let handler = Handler::new(&host.instance.exports, &transaction_handler.handler)?;
            eth_transaction_handlers.insert(transaction_handler.handler.to_owned(), handler);
        }

        Ok(EthereumHandlers {
            block: eth_block_handlers,
            events: eth_event_handlers,
            transaction: eth_transaction_handlers,
        })
    }
}

impl TryFrom<(DatasourceBundle, DatabaseAgent, RpcAgent, ManifestAgent)>
    for DatasourceWasmInstance
{
    type Error = SubgraphError;
    fn try_from(
        value: (DatasourceBundle, DatabaseAgent, RpcAgent, ManifestAgent),
    ) -> Result<Self, Self::Error> {
        let host = AscHost::try_from(value.clone())
            .map_err(|e| SubgraphError::CreateSourceFail(e.to_string()))?;
        let ethereum_handlers = EthereumHandlers::try_from((&host, &value.0.ds))?;
        let name = value.0.name();
        Ok(Self {
            host,
            name,
            ethereum_handlers,
        })
    }
}

impl DatasourceWasmInstance {
    const MAXIMUM_HEAP_SIZE: f32 = 0.5 * (i32::MAX as f32);

    pub fn invoke<T: AscType + AscIndexId>(
        &mut self,
        handler_type: HandlerTypes,
        handler_name: &str,
        data: impl ToAscObj<T>,
    ) -> Result<(), SubgraphError> {
        let handler = match handler_type {
            HandlerTypes::EthereumBlock => self.ethereum_handlers.block.get(handler_name),
            HandlerTypes::EthereumEvent => self.ethereum_handlers.events.get(handler_name),
            HandlerTypes::EthereumTransaction => {
                self.ethereum_handlers.transaction.get(handler_name)
            }
        }
        .ok_or(SubgraphError::InvalidHandlerName(handler_name.to_owned()))?;

        let asc_data = asc_new(&mut self.host, &data)?;
        handler.inner.call(
            &mut self.host.store,
            &[Value::I32(asc_data.wasm_ptr() as i32)],
        )?;

        Ok(())
    }

    pub fn should_reset(&self) -> bool {
        (self.host.current_ptr() as f32) > Self::MAXIMUM_HEAP_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ethereum::transaction::EthereumTransactionReceipt;
    use crate::rpc_client::RpcAgent;
    use crate::runtime::wasm_host::test::mock_wasm_host;
    use num_bigint::BigInt;
    use prometheus::Registry;
    use semver::Version;
    use std::str::FromStr;
    use web3::types::Address;
    use web3::types::Log;
    use web3::types::H160;
    use web3::types::H256;
    use web3::types::U256;
    use web3::types::U64;

    #[test]
    fn test_transaction_receipt_invoke() {
        env_logger::try_init().unwrap_or_default();
        let registry = Registry::default();
        let host = mock_wasm_host(
            Version::parse("0.0.5").unwrap(),
            "../subgraph-testing/packages/v0_0_5/build/TestTransaction/TestTransaction.wasm",
            &registry,
            RpcAgent::new_mock(&registry),
        );
        let tx_handler = Handler::new(&host.instance.exports, "testTransaction").unwrap();
        let mut txs_handlers = HashMap::new();
        txs_handlers.insert("testTransaction".to_string(), tx_handler);
        let mut ds = DatasourceWasmInstance {
            name: "TestTransaction".to_string(),
            host,
            ethereum_handlers: EthereumHandlers {
                block: HashMap::new(),
                transaction: txs_handlers,
                events: HashMap::new(),
            },
        };
        let mut logs = vec![];
        logs.push(Log {
            address: H160::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap(),
            topics: vec![
                H256::from_str(
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                )
                .unwrap(),
                H256::from(
                    Address::from_str("0x9FA7fC17A8d0151F6641a636cC734064E0C64093").unwrap(),
                ),
                H256::from(
                    Address::from_str("0xFEb6A9de89465dA662Ff16F85b5342B73bD0B455").unwrap(),
                ),
            ],
            data: BigInt::from_str("27530167")
                .unwrap()
                .to_signed_bytes_le()
                .into(),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        });
        let tx_receipt = EthereumTransactionReceipt {
            block_number: U64::from(19722402),
            block_hash: H256::from_str(
                "0x2b4e13e0bd4996253f6f7ef66ca2f63ee21de8f29c572dba23f5efbb762dade7",
            )
            .unwrap(),
            transaction_hash: H256::from_str(
                "0xd06f33b2193b1ecd0ed6d8bfb81a4ee31a4c4754dd6f1d42980a0f7da35a14bb",
            )
            .unwrap(),
            transaction_index: U64::from(0),
            cumulative_gas_used: U256::zero(),
            logs,
            ..Default::default()
        };
        ds.invoke(
            HandlerTypes::EthereumTransaction,
            "testTransaction",
            tx_receipt,
        )
        .unwrap();
    }
}
