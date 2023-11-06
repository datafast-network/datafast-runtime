use super::event_filter::EventFilter;
use std::collections::HashMap;

use crate::common::Datasource;
use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::SubgraphData;
use crate::messages::SubgraphJob;
use crate::messages::SubgraphOperationMessage;
use ethabi::Address;
use kanal::AsyncSender;
use web3::types::Log;
use web3::types::H160;

pub type FilterResult<T> = Result<T, FilterError>;

pub trait SubgraphFilter {
    fn filter_event(&self, filter_data: SubgraphData) -> FilterResult<SubgraphData>;

    fn get_contract(&self) -> ethabi::Contract;

    fn get_address(&self) -> &H160;
}

#[derive(Debug, Clone)]
pub enum FilterTypes {
    Events(EventFilter),
}

impl SubgraphFilter for FilterTypes {
    fn filter_event(&self, events: SubgraphData) -> FilterResult<SubgraphData> {
        match self {
            FilterTypes::Events(filter) => filter.filter_event(events),
        }
    }

    fn get_contract(&self) -> ethabi::Contract {
        match self {
            FilterTypes::Events(filter) => filter.get_contract(),
        }
    }

    fn get_address(&self) -> &Address {
        match self {
            FilterTypes::Events(filter) => filter.get_address(),
        }
    }
}

pub struct SubgraphFilterInstance {
    pub filters: HashMap<String, FilterTypes>,
    sources: Vec<Datasource>,
    event_sender: AsyncSender<SubgraphOperationMessage>,
}

impl SubgraphFilterInstance {
    pub fn new(
        config: &ManifestLoader,
        sender: AsyncSender<SubgraphOperationMessage>,
    ) -> Result<Self, FilterError> {
        let sources = config.datasources();
        let filters = sources
            .iter()
            .map(|source| {
                let contract = source.get_abi();
                let address = source.get_address();
                let filter = EventFilter::new(contract, address);
                (source.name.clone(), FilterTypes::Events(filter))
            })
            .collect();
        Ok(Self {
            filters,
            sources,
            event_sender: sender,
        })
    }

    pub fn filter_logs(&self, logs: Vec<Log>) -> Result<Vec<SubgraphJob>, FilterError> {
        //Filter the logs
        let logs_filtered = logs
            .into_iter()
            .filter(|log| {
                self.sources
                    .iter()
                    .any(|source| source.check_log_matches(log))
            })
            .collect::<Vec<_>>();

        let mut jobs = Vec::new();
        for log in logs_filtered.into_iter() {
            //Get the source that matches the log
            //Unwrap is safe because we already filtered the logs
            let source = self
                .sources
                .iter()
                .find(|source| source.check_log_matches(&log))
                .unwrap();
            let handler = source.mapping.get_handler_for_log(log.topics[0]).map_or(
                Err(FilterError::ParseError("No handler found".to_string())),
                Ok,
            )?;
            let filter = self.filters.get(&source.name).ok_or_else(|| {
                FilterError::ParseError(format!("No filter found for source {}", source.name))
            })?;
            let event = filter
                .filter_event(SubgraphData::Log(log))
                .expect("Filter error");
            //TODO: Add block header and transaction into the event
            jobs.push(SubgraphJob {
                source: source.name.clone(),
                handler: handler.handler,
                data: event,
            })
        }
        Ok(jobs)
    }

    pub async fn filter_logs_and_send(&self, logs: Vec<Log>) -> Result<(), FilterError> {
        let events = self
            .filter_logs(logs)?
            .into_iter()
            .map(SubgraphOperationMessage::Job)
            .collect::<Vec<_>>();
        for event in events {
            self.event_sender.send(event).await?;
        }
        Ok(())
    }
}
