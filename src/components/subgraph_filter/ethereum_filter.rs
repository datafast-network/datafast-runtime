use super::utils::check_log_matches;
use super::utils::get_address;
use super::utils::get_handler_for_log;
use super::SubgraphFilterTrait;
use crate::chain::ethereum::event::EthereumEventData;
use crate::common::Chain;
use crate::common::Datasource;
use crate::components::manifest_loader::LoaderTrait;
use crate::components::manifest_loader::ManifestLoader;
use crate::errors::FilterError;
use crate::log_debug;
use crate::log_info;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use ethabi::Contract;
use std::collections::HashMap;
use web3::types::Log;
use web3::types::H160;

#[derive(Debug, Clone)]
pub struct EthereumFilter {
    contracts: HashMap<String, Contract>,
    addresses: HashMap<H160, Datasource>,
}

impl EthereumFilter {
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

        log_debug!(EthereumFilter, "Event found";
            "event" => format!("{:?}", event),
            "log" => format!("{:?}", log));

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

    fn filter_events(&self, logs: Vec<Log>) -> Result<Vec<EthereumFilteredEvent>, FilterError> {
        let mut events = Vec::new();
        for log in logs.iter() {
            let check_log_valid = self
                .addresses
                .iter()
                .any(|(addr, source)| addr == &log.address && check_log_matches(source, log));
            if !check_log_valid {
                continue;
            }
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
            let event = self.parse_event(contract, log)?;
            events.push(EthereumFilteredEvent {
                datasource: source.name.clone(),
                handler: event_handler.handler.clone(),
                event,
            })
        }
        Ok(events)
    }

    //TODO: implement filter_block

    //TODO: implement filter_call_function
}

impl SubgraphFilterTrait for EthereumFilter {
    fn new(_chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let addresses = manifest
            .datasources()
            .iter()
            .map(|source| (get_address(source), source))
            .flat_map(|(address, source)| address.map(|address| (address, source.clone())))
            .collect();
        let contracts = manifest.load_ethereum_contracts()?;
        let filter = EthereumFilter {
            contracts,
            addresses,
        };
        log_info!(EthereumFilter, "Init success";
            Addresses => filter.addresses.len(),
            Contracts => filter.contracts.len()
        );
        Ok(filter)
    }

    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match data {
            SerializedDataMessage::Ethereum { block, logs, .. } => {
                let events = self.filter_events(logs)?;
                log_info!(EthereumFilter, "Filtered events";
                    events => events.len(),
                    block_number => format!("{:?}", block.number)
                );
                Ok(FilteredDataMessage::Ethereum { events, block })
            }
        }
    }
}
