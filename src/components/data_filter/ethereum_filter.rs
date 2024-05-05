use super::utils::get_handler_for_log;
use super::utils::parse_event;
use super::DataFilterTrait;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::chain::ethereum::transaction::EthereumTransactionReceipt;
use crate::common::ABIs;
use crate::common::BlockDataMessage;
use crate::common::Datasource;
use crate::common::EthereumFilteredEvent;
use crate::common::FilteredDataMessage;
use crate::debug;
use crate::errors::FilterError;
use ethabi::Contract;
use web3::types::Log;

#[derive(Debug, Clone)]
struct DatasourceWithContract {
    ds: Datasource,
    contract: Contract,
}

#[derive(Debug, Clone)]
pub struct EthereumFilter {
    ds: Vec<DatasourceWithContract>,
}

impl EthereumFilter {
    pub fn new(datasources: Vec<Datasource>, abis: ABIs) -> Self {
        let ds = datasources
            .into_iter()
            .map(|ds| {
                let abi_name = ds.source.abi.clone();
                let contract = abis.get_contract(&abi_name).unwrap();
                DatasourceWithContract { ds, contract }
            })
            .collect::<Vec<_>>();
        Self { ds }
    }

    fn filter_events(
        &self,
        block_header: EthereumBlockData,
        txs: Vec<EthereumTransactionData>,
        logs: Vec<Log>,
    ) -> Result<Vec<EthereumFilteredEvent>, FilterError> {
        let result = logs
            .into_iter()
            .filter_map(|log| {
                let source = self.ds.iter().find(|s| {
                    s.ds.source
                        .address
                        .as_ref()
                        .map(|addr| {
                            *addr.to_lowercase() == format!("{:?}", log.address).to_lowercase()
                        })
                        .unwrap_or(false)
                });

                if let Some(DatasourceWithContract { ds, contract }) = source {
                    let event_handler = get_handler_for_log(ds, &log.topics[0]);

                    event_handler.as_ref()?;

                    let event_handler = event_handler.unwrap();

                    //Parse the event
                    let tx = txs
                        .get(log.transaction_index.unwrap().as_usize())
                        .cloned()
                        .expect("No Tx found for log");

                    let event = parse_event(contract, log, block_header.to_owned(), tx)
                        .map(|e| EthereumFilteredEvent {
                            event: e,
                            handler: event_handler.handler,
                            datasource: ds.name.clone(),
                        })
                        .expect("Parsing failed");
                    Some(event)
                } else {
                    let tx = txs
                        .get(log.transaction_index.unwrap().as_usize())
                        .cloned()
                        .expect("No Tx found for log");

                    // Try each datasource that comes without Address to see if any match?
                    self.ds
                        .iter()
                        .filter(|ds| ds.ds.source.address.is_none())
                        .find_map(|ds| {
                            parse_event(
                                &ds.contract,
                                log.clone(),
                                block_header.to_owned(),
                                tx.clone(),
                            )
                            .and_then(|e| {
                                let handler = get_handler_for_log(&ds.ds, &log.topics[0]);
                                if let Some(event_handler) = handler {
                                    return Some(EthereumFilteredEvent {
                                        event: e,
                                        handler: event_handler.handler,
                                        datasource: ds.ds.name.clone(),
                                    });
                                }
                                debug!(DataFilter,
                                    "No handler found for log";
                                    log => format!("{:?}", log),
                                    datasource => ds.ds.name.clone(),
                                    block => format!("{:?}", block_header)
                                );
                                None
                            })
                        })
                }
            })
            .collect::<Vec<_>>();

        Ok(result)
    }

    fn collect_txs(
        &self,
        block_header: &EthereumBlockData,
        transactions: &Vec<EthereumTransactionData>,
        logs: &Vec<Log>,
    ) -> Result<Vec<EthereumTransactionReceipt>, FilterError> {
        let has_tx_handlers = self
            .ds
            .iter()
            .any(|ds| ds.ds.mapping.transactionHandlers.is_some());
        if has_tx_handlers {
            let txs = transactions
                .iter()
                .map(|tx| EthereumTransactionReceipt::from((block_header, tx.clone(), logs)))
                .collect::<Vec<_>>();
            Ok(txs)
        } else {
            Ok(vec![])
        }
    }

    fn collect_events(
        &self,
        block_header: EthereumBlockData,
        txs: Vec<EthereumTransactionData>,
        logs: Vec<Log>,
    ) -> Result<Vec<EthereumFilteredEvent>, FilterError> {
        let has_event_handlers = self
            .ds
            .iter()
            .any(|ds| ds.ds.mapping.eventHandlers.is_some());
        if has_event_handlers {
            self.filter_events(block_header, txs, logs)
        } else {
            Ok(vec![])
        }
    }

    // TODO: implement filter_block

    // TODO: implement filter_call_function
}

impl DataFilterTrait for EthereumFilter {
    fn handle_serialize_message(
        &self,
        data: BlockDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match data {
            BlockDataMessage::Ethereum {
                block,
                logs,
                transactions,
            } => {
                let txs = self.collect_txs(&block, &transactions, &logs)?;
                let events = self.collect_events(block.clone(), transactions, logs)?;
                Ok(FilteredDataMessage::Ethereum { events, block, txs })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::components::ManifestAgent;

    fn erc20_contract() -> Contract {
        let erc20_abi = r#"
[{"constant":true,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"_spender","type":"address"},{"name":"_value","type":"uint256"}],"name":"approve","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[],"name":"totalSupply","outputs":[{"name":"","type":"uint256"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"_from","type":"address"},{"name":"_to","type":"address"},{"name":"_value","type":"uint256"}],"name":"transferFrom","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint256"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_owner","type":"address"}],"name":"balanceOf","outputs":[{"name":"balance","type":"uint256"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"_to","type":"address"},{"name":"_value","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[{"name":"_owner","type":"address"},{"name":"_spender","type":"address"}],"name":"allowance","outputs":[{"name":"","type":"uint256"}],"payable":false,"stateMutability":"view","type":"function"},{"payable":true,"stateMutability":"payable","type":"fallback"},{"anonymous":false,"inputs":[{"indexed":true,"name":"owner","type":"address"},{"indexed":true,"name":"spender","type":"address"},{"indexed":false,"name":"value","type":"uint256"}],"name":"Approval","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"from","type":"address"},{"indexed":true,"name":"to","type":"address"},{"indexed":false,"name":"value","type":"uint256"}],"name":"Transfer","type":"event"}]
"#;
        serde_json::from_str(erc20_abi).unwrap()
    }

    #[tokio::test]
    async fn test_parsing_logs() {
        env_logger::try_init().unwrap_or_default();

        let logs = r#"
[
  {
    "address": "0x8e870d67f660d95d5be530380d0ec0bd388289e1",
    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
    "blockNumber": "0x989680",
    "data": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "logIndex": "0x2b",
    "removed": false,
    "topics": [
      "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925",
      "0x000000000000000000000000903171964ee615dc99f350bd29ea747b887ae3f4",
      "0x0000000000000000000000008a91c9a16cd62693649d80afa85a09dbbdcb8508"
    ],
    "transactionHash": "0x9dab17b59a0612347929fe03bb82d6a03f8a0880ac6201eb992c4f3b4fb4d088",
    "transactionIndex": "0x0"
  },
  {
    "address": "0x8e870d67f660d95d5be530380d0ec0bd388289e1",
    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
    "blockNumber": "0x989680",
    "data": "0x000000000000000000000000000000000000000000000001a055690d9db80000",
    "logIndex": "0x2c",
    "removed": false,
    "topics": [
      "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
      "0x0000000000000000000000008a91c9a16cd62693649d80afa85a09dbbdcb8508",
      "0x000000000000000000000000903171964ee615dc99f350bd29ea747b887ae3f4"
    ],
    "transactionHash": "0x9dab17b59a0612347929fe03bb82d6a03f8a0880ac6201eb992c4f3b4fb4d088",
    "transactionIndex": "0x0"
  },
  {
    "address": "0x8e870d67f660d95d5be530380d0ec0bd388289e1",
    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
    "blockNumber": "0x989680",
    "data": "0x000000000000000000000000000000000000000000000001a055690d9db80000",
    "logIndex": "0x2d",
    "removed": false,
    "topics": [
      "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
      "0x000000000000000000000000903171964ee615dc99f350bd29ea747b887ae3f4",
      "0x0000000000000000000000002b7f0e2dcddd255ba21e2327fcd0007e3416f3f4"
    ],
    "transactionHash": "0x9dab17b59a0612347929fe03bb82d6a03f8a0880ac6201eb992c4f3b4fb4d088",
    "transactionIndex": "0x0"
  },
  {
    "address": "0x8a91c9a16cd62693649d80afa85a09dbbdcb8508",
    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
    "blockNumber": "0x989680",
    "data": "0x00000000000000000000000000000000000000000000000000000000000000010000000000000000000000002b7f0e2dcddd255ba21e2327fcd0007e3416f3f4000000000000000000000000000000000000000000000001a055690d9db8000000000000000000000000000000000000000000000000000000000004c674de80000000000000000000000000000000000000000000000000000000005eb01705",
    "logIndex": "0x2e",
    "removed": false,
    "topics": [
      "0xddaecf3d7bee1a5fcc29f2e0b87f0b1b932dbac3bcd7ea572e24d4ba49c0bba3"
    ],
    "transactionHash": "0x9dab17b59a0612347929fe03bb82d6a03f8a0880ac6201eb992c4f3b4fb4d088",
    "transactionIndex": "0x0"
  },
  {
    "address": "0xdac17f958d2ee523a2206206994597c13d831ec7",
    "blockHash": "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe",
    "blockNumber": "0x989680",
    "data": "0x000000000000000000000000000000000000000000000000000000009f280a06",
    "logIndex": "0x86",
    "removed": false,
    "topics": [
      "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
      "0x0000000000000000000000009c43d6a63f7fd0ae469ec0aeda2a90be93038f59",
      "0x00000000000000000000000096e9a06a22d4445a757dfe9b4ff2c77a12dd60f2"
    ],
    "transactionHash": "0xf9084755ea9905d54a61b1109626ad3de5e8c2edf3b9f7a42831037ace6f2456",
    "transactionIndex": "0x0"
  }
]
"#;
        let logs: Vec<Log> = serde_json::from_str(logs).unwrap();
        let contract = erc20_contract();
        let test_manifest = ManifestAgent::new("fs://Users/vutran/Desktop/build")
            .await
            .unwrap();
        let datasources_1: Vec<Datasource> = test_manifest.datasources().into();

        let mut filter = EthereumFilter::new(datasources_1.clone(), ABIs::default());
        let header = EthereumBlockData::default();
        let txs = vec![EthereumTransactionData::default()];

        // Parse ERC20 events without caring about address
        let events = logs
            .clone()
            .into_iter()
            .filter_map(|log| parse_event(&contract, log, header.clone(), txs[0].clone()))
            .collect::<Vec<_>>();

        assert_eq!(events.len(), 4);

        // Use USDT Address to filter
        let events = filter
            .filter_events(header.clone(), txs.clone(), logs.clone())
            .unwrap();
        assert_eq!(events.len(), 1);

        // Use no Address
        filter.ds[0].ds.source.address = None;
        let events = filter
            .filter_events(header.clone(), txs.clone(), logs.clone())
            .unwrap();
        assert_eq!(events.len(), 4);

        // Use Paxos:USDP Address to filter
        filter.ds[0].ds.source.address =
            Some("0x8E870D67F660D95d5be530380D0eC0bd388289E1".to_string());
        let events = filter
            .filter_events(header.clone(), txs.clone(), logs.clone())
            .unwrap();
        assert_eq!(events.len(), 3);

        // Use both contracts
        let mut ds_usdt = filter.ds[0].clone();
        ds_usdt.ds.source.address = Some("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string());
        filter.ds.push(ds_usdt);
        let events = filter.filter_events(header, txs, logs).unwrap();
        assert_eq!(events.len(), 4);
        // TX hash of this transfer: 0xf9084755ea9905d54a61b1109626ad3de5e8c2edf3b9f7a42831037ace6f2456
        assert_eq!(
            events
                .last()
                .unwrap()
                .event
                .params
                .last()
                .cloned()
                .unwrap()
                .value
                .into_uint()
                .unwrap()
                .as_usize(),
            2670201350
        );
    }
}
