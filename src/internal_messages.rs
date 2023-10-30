use crate::asc::native_types::store::StoreValueKind;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
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

#[derive(Debug)]
pub enum StoreOperationMessage {
    Create,
    Load(String),
    Update,
    Delete(String),
}

#[derive(Debug)]
pub enum StoreRequestResult {
    Create(String),
    Load(HashMap<String, StoreValueKind>),
    Delete,
    Update,
}
