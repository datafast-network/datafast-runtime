use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::Datasource;
use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::SubgraphData;
use crate::messages::SubgraphJob;
use crate::messages::SubgraphOperationMessage;
use crate::subgraph_filter::filter_instance::FilterData;
use crate::subgraph_filter::filter_instance::SubgraphFilter;
use ethabi::Contract;
use std::collections::HashMap;
use web3::types::Log;
use web3::types::H160;

pub type EthereumBlockFilter = (EthereumBlockData, Vec<EthereumTransactionData>, Vec<Log>);

#[derive(Debug, Clone)]
pub struct EthereumLogFilter {
    contracts: HashMap<String, Contract>,
    sources: Vec<Datasource>,
    addresses: HashMap<H160, Datasource>,
}

impl EthereumLogFilter {
    pub fn new(manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let sources = manifest.datasources().clone();
        let mut contracts = HashMap::new();
        for source in sources.iter() {
            if let Some(abi) = manifest.get_abi(&source.name, &source.get_abi_name()) {
                let contract = serde_json::from_value(abi.clone())?;
                contracts.insert(source.name.clone(), contract);
            }
        }

        //Map addresses to sources
        let addresses = sources
            .iter()
            .map(|source| (source.get_address(), source))
            .flat_map(|(address, source)| address.map(|address| (address, source.clone())))
            .collect();

        Ok(Self {
            contracts,
            sources,
            addresses,
        })
    }

    fn parse_event(
        &self,
        contract: &Contract,
        log: &Log,
    ) -> Result<EthereumEventData, FilterError> {
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
                log_index: log.log_index.unwrap_or_default(),
                transaction_log_index: log.transaction_log_index.unwrap_or_default(),
                log_type: log.log_type.clone(),
                ..Default::default()
            })
            .map_err(|e| FilterError::ParseError(e.to_string()))
    }

    pub fn filter_event_logs(
        &self,
        block: EthereumBlockFilter,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError> {
        let (block, transactions, logs) = block;

        //Filter the logs
        let logs_filtered = logs
            .into_iter()
            .filter(|log| self.addresses.iter().any(|(addr, _)| addr == &log.address))
            .filter(|log| {
                self.addresses
                    .get(&log.address)
                    .map_or(false, |source| source.check_log_matches(log))
            })
            .collect::<Vec<_>>();

        let mut jobs = Vec::new();
        for log in logs_filtered.into_iter() {
            //Get the source that matches the log
            //Unwrap is safe because we already filtered the logs
            let source = self.addresses.get(&log.address).unwrap();

            //Get the handler for the log
            let handler = source.mapping.get_handler_for_log(log.topics[0]).map_or(
                Err(FilterError::ParseError("No handler found".to_string())),
                Ok,
            )?;
            let contract = self
                .contracts
                .get(&source.name)
                .ok_or_else(|| FilterError::ParseError("No contract found".to_string()))?;

            //Parse the event
            let mut event = self.parse_event(contract, &log)?;

            //Get the transaction hash for the log
            let tx_hash = log
                .transaction_hash
                .ok_or_else(|| FilterError::ParseError("No transaction hash found".to_string()))?;

            event.block = block.clone();
            event.transaction = transactions
                .iter()
                .find(|tx| tx.hash == tx_hash)
                .map_or(EthereumTransactionData::default(), |tx| tx.clone());
            let job = SubgraphJob {
                source: source.name.clone(),
                handler: handler.handler,
                data: SubgraphData::Event(event),
            };
            jobs.push(SubgraphOperationMessage::Job(job));
        }
        Ok(jobs)
    }

    //TODO: implement filter_block

    //TODO: implement filter_call_function
}

impl SubgraphFilter for EthereumLogFilter {
    fn filter_events(
        &self,
        filter_data: FilterData,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError> {
        match filter_data {
            FilterData::Events(block) => self.filter_event_logs(block),
        }
    }
}