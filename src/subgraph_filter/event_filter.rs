use crate::subgraph_filter::filter::SubgraphFilter;

#[derive(Clone, Debug)]
pub struct EventFilter {
    contract: ethabi::Contract,
    address: ethabi::Address,
}

impl EventFilter {
    pub fn new(contract: ethabi::Contract, address: ethabi::Address) -> Self {
        Self { contract, address }
    }
}

impl SubgraphFilter for EventFilter {
    fn get_contract(&self) -> ethabi::Contract {
        self.contract.clone()
    }

    fn get_address(&self) -> &ethabi::Address {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use crate::subgraph_filter::event_filter::EventFilter;
    use crate::subgraph_filter::filter::SubgraphFilter;
    use std::fs::File;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_parse_event() {
        env_logger::try_init().unwrap_or_default();
        let abi = File::open("/Users/quannguyen/workspace/myjob/subgraph-wasm-runtime/src/subgraph_filter/ERC20.json").unwrap();
        let contract = ethabi::Contract::load(abi).unwrap();
        let address =
            ethabi::Address::from_str("0x95a41fb80ca70306e9ecf4e51cea31bd18379c18").unwrap();
        assert_eq!(contract.events.len(), 2);
        let event_filter = EventFilter::new(contract, address);
        let block_json = File::open("/Users/quannguyen/workspace/myjob/subgraph-wasm-runtime/src/subgraph_filter/block_10000000.json").unwrap();
        let block = serde_json::from_reader(block_json).unwrap();
        let events = event_filter.filter_log(&block).await.unwrap();
        assert_eq!(events.len(), 5);
        let first_event = events.first().unwrap();
        assert_eq!(first_event.params.len(), 3);
    }
}
