use super::utils::check_log_matches;
use super::utils::get_address;
use super::utils::get_handler_for_log;
use super::SubgraphFilterTrait;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::Chain;
use crate::common::Datasource;
use crate::components::manifest_loader::ManifestLoader;
use crate::error;
use crate::errors::FilterError;
use crate::info;
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
    // FIXME: passing ref to a Component is undesirable, only get what you actually need
    pub fn new(_chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError> {
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
        info!(EthereumFilter, "Init success";
            Addresses => filter.addresses.len(),
            Contracts => filter.contracts.len()
        );
        Ok(filter)
    }

    fn parse_event(
        &self,
        contract: &Contract,
        log: Log,
        block_header: EthereumBlockData,
        transaction: EthereumTransactionData,
    ) -> Result<EthereumEventData, FilterError> {
        let block_number = block_header.number;
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
                log_type: log.log_type,
                block: block_header,
                transaction,
            })
            .map_err(|e| {
                error!(
                    EthereumFilter,
                    "parse event error";
                    error => e,
                    block_number => format!("{:?}", block_number)
                );
                FilterError::ParseError(e.to_string())
            })
    }

    fn filter_events(
        &self,
        block_header: EthereumBlockData,
        txs: Vec<EthereumTransactionData>,
        logs: Vec<Log>,
    ) -> Result<Vec<EthereumFilteredEvent>, FilterError> {
        let mut events = Vec::new();
        for log in logs {
            let check_log_valid = self
                .addresses
                .iter()
                .any(|(addr, source)| addr == &log.address && check_log_matches(source, &log));
            if !check_log_valid {
                continue;
            }
            //Unwrap is safe because we already filtered the logs
            let source = self.addresses.get(&log.address).unwrap();

            //Get the handler for the log
            let event_handler = get_handler_for_log(source, &log.topics[0])
                .ok_or(FilterError::ParseError("No handler found".to_string()))?;

            let contract = self
                .contracts
                .get(&source.name)
                .ok_or(FilterError::ParseError("No contract found".to_string()))?;

            //Parse the event
            let tx = txs
                .get(log.transaction_index.unwrap().as_usize())
                .cloned()
                .ok_or(FilterError::TxNotFound)?;

            let event = self.parse_event(contract, log, block_header.to_owned(), tx)?;

            events.push(EthereumFilteredEvent {
                datasource: source.name.clone(),
                handler: event_handler.handler.clone(),
                event,
            })
        }

        Ok(events)
    }

    // TODO: implement filter_block

    // TODO: implement filter_call_function
}

impl SubgraphFilterTrait for EthereumFilter {
    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match data {
            SerializedDataMessage::Ethereum {
                block,
                logs,
                transactions,
            } => {
                let events = self.filter_events(block.clone(), transactions, logs)?;
                Ok(FilteredDataMessage::Ethereum { events, block })
            }
        }
    }
}
