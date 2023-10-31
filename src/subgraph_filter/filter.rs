use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::protobuf as pb;
use crate::subgraph_filter::errors::FilterError;
use crate::subgraph_filter::FilterResult;

#[async_trait::async_trait]
pub trait SubgraphFilter {
    async fn filter(
        &self,
        block_data: &pb::ethereum::Block,
    ) -> FilterResult<Vec<EthereumEventData>> {
        let eth_block = EthereumBlockData::from(block_data.clone());
        let logs = block_data
            .logs
            .clone()
            .into_iter()
            .map(web3::types::Log::from)
            .collect::<Vec<_>>();
        let mut events = Vec::new();
        for raw_log in logs.iter() {
            match self.parse_event(raw_log) {
                Ok(mut data) => {
                    data.block = eth_block.clone();
                    let transaction = block_data.transactions.iter().find_map(|tx| {
                        if tx.hash == raw_log.transaction_hash.unwrap().to_string() {
                            Some(EthereumTransactionData::from(tx.clone()))
                        } else {
                            None
                        }
                    });
                    data.transaction = transaction.unwrap();
                    events.push(data);
                }
                Err(e) => {
                    log::error!("Error parsing event: {:?}", e)
                }
            }
        }
        Ok(events)
    }

    fn parse_event(&self, log: &web3::types::Log) -> FilterResult<EthereumEventData> {
        let contract = self.get_contract();
        let event = contract
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
                log_index: log.log_index.unwrap(),
                transaction_log_index: log.transaction_log_index.unwrap(),
                log_type: log.log_type.clone(),
                ..Default::default()
            })
            .map_err(|e| FilterError::ParseError(e.to_string()))
    }

    fn get_contract(&self) -> ethabi::Contract;

    fn get_address(&self) -> &ethabi::Address;
}
