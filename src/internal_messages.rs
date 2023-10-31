use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::db_worker::abstract_types::Value;
use std::collections::HashMap;
use web3::types::Log;

#[derive(Debug)]
pub enum SubgraphData {
    Block(EthereumBlockData),
    Transaction(EthereumTransactionData),
    Event(EthereumEventData),
    Log(Log),
}

#[derive(Debug)]
pub struct SubgraphJob {
    pub source: String,
    pub handler: String,
    pub data: SubgraphData,
}

#[derive(Debug)]
pub enum SubgraphOperationMessage {
    Job(SubgraphJob),
    Finish,
}

pub type EntityType = String;
pub type EntityID = String;

#[derive(Debug)]
pub enum StoreOperationMessage {
    Create((EntityType, HashMap<String, Value>)),
    Load((EntityType, EntityID)),
    Update((EntityType, EntityID, HashMap<String, Value>)),
    Delete((EntityType, EntityID)),
}

#[derive(Debug)]
pub enum StoreRequestResult {
    Create(String),
    Load(Option<HashMap<String, Value>>),
    Delete,
    Update,
}
