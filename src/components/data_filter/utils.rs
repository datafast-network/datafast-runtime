use df_types::chain::ethereum::block::EthereumBlockData;
use df_types::chain::ethereum::event::EthereumEventData;
use df_types::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::Datasource;
use crate::common::EventHandler;
use ethabi::Contract;
use tiny_keccak::Hasher;
use df_types::web3::types::Log;
use df_types::web3::types::H256;

pub fn parse_event(
    contract: &Contract,
    log: Log,
    block_header: EthereumBlockData,
    transaction: EthereumTransactionData,
) -> Option<EthereumEventData> {
    if log.topics.is_empty() {
        return None;
    }

    let event = contract
        .events()
        .find(|event| event.signature() == log.topics[0]);

    event?;

    let event = event.unwrap();

    event
        .parse_log(ethabi::RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        })
        .map(|event| EthereumEventData {
            params: event.params,
            address: log.address,
            log_index: log.log_index.unwrap_or_default(),
            transaction_log_index: log.transaction_log_index.unwrap_or_default(),
            log_type: log.log_type,
            block: block_header,
            transaction,
        })
        .ok()
}

pub fn get_handler_for_log(source: &Datasource, topic0: &H256) -> Option<EventHandler> {
    source
        .mapping
        .eventHandlers
        .clone()
        .and_then(|event_handlers| {
            event_handlers
                .iter()
                .find(|handler| &parse_topic0_event(&handler.event) == topic0)
                .cloned()
        })
}

fn parse_topic0_event(handler: &str) -> H256 {
    let mut result = [0u8; 32];
    let data = handler.replace("indexed", "").replace(' ', "").into_bytes();
    let mut sponge = tiny_keccak::Keccak::v256();
    sponge.update(&data);
    sponge.finalize(&mut result);
    H256::from_slice(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use df_logger::loggers::init_logger;
    use std::str::FromStr;

    #[test]
    fn test_convert_event_name_to_topic0() {
        init_logger();
        let event_handler = EventHandler {
            event: "Transfer(indexed address,indexed address,uint256)".to_string(),
            handler: "handleTransfer".to_string(),
        };
        assert_eq!(
            parse_topic0_event(event_handler.event.as_str()),
            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                .unwrap()
        );
    }
}
