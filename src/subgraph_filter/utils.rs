use crate::common::Datasource;
use crate::common::EventHandler;
use tiny_keccak::Hasher;
use web3::types::Address;
use web3::types::Log;
use web3::types::H256;
use web3::types::U64;

pub fn check_log_matches(source: &Datasource, raw_log: &Log) -> bool {
    if get_start_block(source) > raw_log.block_number {
        return false;
    }

    //Check topic0 matches event handler
    match raw_log.topics.first() {
        None => false,
        Some(topic) => get_handler_for_log(source, topic).is_some(),
    }
}

pub fn get_handler_for_log(source: &Datasource, topic0: &H256) -> Option<EventHandler> {
    if let Some(event_handlers) = source.mapping.eventHandlers.clone() {
        return event_handlers
            .iter()
            .find(|handler| &parse_topic0_event(&handler.event) == topic0)
            .cloned();
    }
    None
}

fn get_start_block(source: &Datasource) -> Option<U64> {
    source.source.startBlock.map(U64::from)
}

pub fn get_address(source: &Datasource) -> Option<Address> {
    match source.source.address.clone() {
        None => None,
        Some(address) => address.parse().ok(),
    }
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
    use std::str::FromStr;
    #[test]
    fn test_convert_event_name_to_topic0() {
        env_logger::try_init().unwrap_or_default();
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
