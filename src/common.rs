use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use tiny_keccak::Hasher;
use web3::types::Address;
use web3::types::Log;
use web3::types::H256;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MappingABI {
    pub name: String,
    pub file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct EventHandler {
    pub event: String,
    pub handler: String,
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BlockHandler {
    pub filter: Option<String>,
    pub handler: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct Mapping {
    pub kind: String,
    pub apiVersion: Version,
    pub entities: Vec<String>,
    pub abis: Vec<MappingABI>,
    pub eventHandlers: Option<Vec<EventHandler>>,
    pub blockHandlers: Option<Vec<BlockHandler>>,
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Datasource {
    pub kind: String,
    pub name: String,
    pub network: String,
    pub source: HashMap<String, String>,
    pub mapping: Mapping,
}

impl Datasource {
    pub fn check_log_matches(&self, raw_log: &Log) -> bool {
        if self.get_start_block() > raw_log.block_number {
            return false;
        }
        let has_event = match raw_log.topics.first() {
            None => false,
            Some(topic) => self.mapping.get_handler_for_log(*topic).is_some(),
        };

        has_event && self.get_address() == raw_log.address
    }

    pub fn get_handler_for_log(&self, topic0: H256) -> Option<EventHandler> {
        self.mapping.get_handler_for_log(topic0)
    }

    pub fn get_start_block(&self) -> Option<web3::types::U64> {
        self.source
            .get("startBlock")
            .map(|block| block.parse::<u64>().unwrap_or_default())
            .map(web3::types::U64::from)
    }

    pub fn get_network_chain(&self) -> Option<u64> {
        self.source
            .get("network")
            .map(|network| network.parse::<u64>().unwrap_or_default())
    }

    pub fn get_address(&self) -> Address {
        self.source
            .get("address")
            .unwrap()
            .parse::<Address>()
            .unwrap_or_default()
    }

    pub fn get_abi_name(&self) -> String {
        self.source.get("abi").expect("ABI not found").clone()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default, Clone)]
#[allow(non_snake_case)]
pub struct SubgraphYaml {
    pub dataSources: Vec<Datasource>,
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
