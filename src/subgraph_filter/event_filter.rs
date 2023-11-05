use crate::chain::ethereum::event::EthereumEventData;
use crate::errors::FilterError;
use crate::subgraph_filter::filter::FilterData;
use crate::subgraph_filter::filter::FilterResult;
use crate::subgraph_filter::filter::SubgraphFilter;
use std::str::FromStr;
use web3::types::Log;

#[derive(Clone, Debug)]
pub struct SubgraphLogData {
    pub name: String,
    pub data: EthereumEventData,
}

#[derive(Clone, Debug)]
pub struct EventFilter {
    contract: ethabi::Contract,
    address: ethabi::Address,
}

impl EventFilter {
    pub fn new(contract: ethabi::Contract, address: ethabi::Address) -> Self {
        Self { contract, address }
    }

    fn parse_event(&self, log: &Log) -> Result<EthereumEventData, FilterError> {
        let event = self
            .contract
            .events()
            .find(|event| event.signature() == log.topics[0])
            .ok_or(FilterError::ParseError(format!(
                "Invalid signature event {}",
                log.address
            )))?;
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
                log_type: log.log_type.clone(),
                ..Default::default()
            })
            .map_err(|e| FilterError::ParseError(e.to_string()))
    }
}
#[async_trait::async_trait]
impl SubgraphFilter for EventFilter {
    async fn filter_log(&self, data: FilterData) -> FilterResult<FilterData> {
        let logs = data
            .get_logs()
            .into_iter()
            .filter(|log| log == self.get_address())
            .map(Log::from)
            .collect::<Vec<_>>();
        let mut events = Vec::new();
        for raw_log in logs.iter() {
            match self.parse_event(raw_log) {
                Ok(data) => events.push(data),
                Err(e) => {
                    log::error!("Error parsing event: {:?} from log: {:?}", e, raw_log);
                    //panic if critical with main stream flow
                }
            }
        }
        Ok(FilterData::EthereumEventData(events))
    }

    fn get_contract(&self) -> ethabi::Contract {
        self.contract.clone()
    }

    fn get_address(&self) -> &ethabi::Address {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::str::FromStr;

    #[tokio::test]
    //from event: https://etherscan.io/tx/0xf470b475f31530211d9daa18279cca8a36c63136cc2c9aa43e657589da6d5f5d#eventlog
    async fn test_parse_event() {
        env_logger::try_init().unwrap_or_default();
        let abi = File::open("./src/subgraph_filter/ERC20.json").unwrap();
        let contract = ethabi::Contract::load(abi).unwrap();
        let address =
            ethabi::Address::from_str("0x95a41fb80ca70306e9ecf4e51cea31bd18379c18").unwrap();
        assert_eq!(contract.events.len(), 3);
        let event_filter = EventFilter::new(contract, address);
        let block_json = File::open("./src/subgraph_filter/block_10000000.json").unwrap();
        let block = serde_json::from_reader(block_json).unwrap();
        let events = event_filter.filter_log(&block).await.unwrap();
        assert_eq!(events.len(), 5);
        let first_event = events[0].clone();
        assert_eq!(first_event.name, "Transfer");
        assert_eq!(first_event.data.params.len(), 3);
        //asert from address
        let first_params = first_event.data.params.first().unwrap();
        let from = first_params.value.clone().into_address().unwrap();
        let expected_from =
            ethabi::Address::from_str("0x22F0039e614eBA9c51A70376df72B9Ea92cE2500").unwrap();
        assert_eq!(from, expected_from);
        //assert to address
        let second_params = first_event.data.params.get(1).unwrap();
        let to = second_params.value.clone().into_address().unwrap();
        assert_eq!(
            to,
            ethabi::Address::from_str("0x2590918786B30fD27c4E9F1d5b9C8A5F2F7c2754").unwrap()
        );

        //assert value of event
        let last_params = first_event.data.params.last().unwrap();
        let value = last_params.value.clone();
        assert_eq!(
            value.into_uint().unwrap().to_string(),
            "517332400000000000000000"
        );
    }
}
