use crate::common::Datasource;
use crate::common::EventHandler;
use tiny_keccak::Hasher;
use web3::types::H256;

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
