use crate::common::Datasource;
use crate::common::EventHandler;
use crate::common::Mapping;
use tiny_keccak::Hasher;
use web3::types::Address;
use web3::types::Log;
use web3::types::H256;
use web3::types::U64;

impl Datasource {
    pub fn check_log_matches(&self, raw_log: &Log) -> bool {
        if self.get_start_block() > raw_log.block_number {
            return false;
        }

        //Check topic0 matches event handler
        match raw_log.topics.first() {
            None => false,
            Some(topic) => self.mapping.get_handler_for_log(*topic).is_some(),
        }
    }

    pub fn get_handler_for_log(&self, topic0: H256) -> Option<EventHandler> {
        self.mapping.get_handler_for_log(topic0)
    }

    pub fn get_start_block(&self) -> Option<U64> {
        self.source
            .get("startBlock")
            .map(|block| block.parse::<u64>().unwrap_or_default())
            .map(U64::from)
    }

    pub fn get_network_chain(&self) -> Option<u64> {
        self.source
            .get("network")
            .map(|network| network.parse::<u64>().unwrap_or_default())
    }

    pub fn get_address(&self) -> Option<Address> {
        self.source.get("address").unwrap().parse::<Address>().ok()
    }

    pub fn get_abi_name(&self) -> String {
        self.source.get("abi").expect("ABI not found").clone()
    }
}

impl Mapping {
    pub fn get_handler_for_log(&self, topic0: H256) -> Option<EventHandler> {
        if let Some(event_handlers) = self.eventHandlers.clone() {
            return event_handlers
                .iter()
                .find(|handler| handler.get_topic0() == topic0)
                .cloned();
        }
        //TODO: check if block handler matches
        None
    }

    pub fn get_abi_file(&self, abi_name: &str) -> Option<String> {
        self.abis
            .iter()
            .find(|abi| abi.name == abi_name)
            .map(|abi| abi.file.clone())
    }
}

impl EventHandler {
    pub fn get_topic0(&self) -> H256 {
        let mut result = [0u8; 32];
        let data = self
            .event
            .replace("indexed", "")
            .replace(' ', "")
            .into_bytes();
        let mut sponge = tiny_keccak::Keccak::v256();
        sponge.update(&data);
        sponge.finalize(&mut result);
        H256::from_slice(&result)
    }
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
            event_handler.get_topic0(),
            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                .unwrap()
        );
    }
}
