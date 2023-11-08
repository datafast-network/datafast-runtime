use crate::chain::ethereum::event::EthereumEventData;
use crate::common::Datasource;
use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use crate::subgraph_filter::data_source_reader::check_log_matches;
use crate::subgraph_filter::data_source_reader::get_abi_name;
use crate::subgraph_filter::data_source_reader::get_address;
use crate::subgraph_filter::data_source_reader::get_handler_for_log;
use ethabi::Contract;
use std::collections::HashMap;
use web3::types::Log;
use web3::types::H160;

#[derive(Debug, Clone)]
pub struct EthereumLogFilter {
    contracts: HashMap<String, Contract>,
    addresses: HashMap<H160, Datasource>,
}

impl EthereumLogFilter {
    pub fn new(manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let sources = manifest.datasources();
        let mut contracts = HashMap::new();
        for source in sources.iter() {
            if let Some(abi) = manifest.get_abi(&source.name, &get_abi_name(source)) {
                let contract = serde_json::from_value(abi)?;
                contracts.insert(source.name.clone(), contract);
            }
        }

        //Map addresses to sources
        let addresses = sources
            .iter()
            .map(|source| (get_address(source), source))
            .flat_map(|(address, source)| address.map(|address| (address, source.clone())))
            .collect();

        Ok(Self {
            contracts,
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

    pub fn filter_events(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match data {
            SerializedDataMessage::Ethereum { block, logs, .. } => {
                let start = tokio::time::Instant::now();
                let logs_len = logs.len();
                //Filter the logs by address
                let logs_filtered = logs
                    .into_iter()
                    .filter(|log| {
                        self.addresses.iter().any(|(addr, source)| {
                            addr == &log.address && check_log_matches(source, log)
                        })
                    })
                    .collect::<Vec<_>>();

                let mut events = Vec::new();
                for log in logs_filtered.into_iter() {
                    //Unwrap is safe because we already filtered the logs
                    let source = self.addresses.get(&log.address).unwrap();

                    //Get the handler for the log
                    let event_handler = get_handler_for_log(source, &log.topics[0]).map_or(
                        Err(FilterError::ParseError("No handler found".to_string())),
                        Ok,
                    )?;

                    let contract = self
                        .contracts
                        .get(&source.name)
                        .ok_or_else(|| FilterError::ParseError("No contract found".to_string()))?;

                    //Parse the event
                    let event = self.parse_event(contract, &log)?;
                    events.push(EthereumFilteredEvent {
                        datasource: source.name.clone(),
                        handler: event_handler.handler.clone(),
                        event,
                    })
                }
                log::info!(
                    "filtered {} events in {} logs for block_number {:?} - time: {:?}",
                    events.len(),
                    logs_len,
                    block.number,
                    start.elapsed()
                );
                Ok(FilteredDataMessage::Ethereum { events, block })
            }
        }
    }

    //TODO: implement filter_block

    //TODO: implement filter_call_function
}
