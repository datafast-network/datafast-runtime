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
use web3::futures::TryFutureExt;
use web3::types::Log;
use web3::types::H160;

pub type FilterResult<T> = Result<T, FilterError>;

pub trait SubgraphFilter {
    fn filter_events(&self, filter_data: SubgraphData) -> FilterResult<SubgraphData>;

    fn get_contract(&self) -> ethabi::Contract;

    fn get_address(&self) -> &H160;
}

#[derive(Debug, Clone)]
pub enum FilterTypes {
    Events(EventFilter),
}

impl SubgraphFilter for FilterTypes {
    fn filter_events(&self, events: SubgraphData) -> FilterResult<SubgraphData> {
        match self {
            FilterTypes::Events(filter) => filter.filter_events(events),
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
        //Get all sources that match the log
        let sources = logs
            .iter()
            .filter_map(|log| {
                self.sources
                    .iter()
                    .find(|source| source.check_log_matches(log))
            })
            .collect::<Vec<_>>();

        //Get the filter for the source
        let filter = sources
            .iter()
            .find_map(|source| self.filters.get(&source.name).cloned());

        //Filter the logs
        let jobs = logs
            .into_iter()
            .filter(|log| sources.iter().any(|source| source.check_log_matches(log)))
            .map(|log| {
                let source = sources
                    .iter()
                    .find(|source| source.check_log_matches(&log))
                    .unwrap();
                let handler = source.mapping.get_handler_for_log(log.topics[0]);
                SubgraphJob {
                    source: source.name.clone(),
                    handler: handler.unwrap().handler,
                    data: SubgraphData::Log(log),
                }
            })
            .collect::<Vec<_>>();
        let mut events = Vec::new();

        //Filter the events
        match filter {
            None => return Err(FilterError::ParseError("No filter found".to_string())),
            Some(filter) => {
                for job in jobs.into_iter() {
                    let event = filter.filter_events(job.data.clone())?;
                    let new = SubgraphJob { data: event, ..job };
                    events.push(new);
                }
            }
        };
        Ok(events)
    }

    pub async fn filter_logs_and_send(&self, logs: Vec<Log>) -> Result<(), FilterError> {
        let events = self
            .filter_logs(logs)?
            .iter()
            .map(|job| SubgraphOperationMessage::Job(job.clone()))
            .collect::<Vec<_>>();
        for event in events {
            self.event_sender.send(event).await?;
        }
        Ok(())
    }
}
