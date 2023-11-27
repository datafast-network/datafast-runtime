use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::BlockPtr;
use crate::runtime::asc::native_types::store::Value;
use std::collections::HashMap;
use web3::types::Log;

#[derive(Debug)]
pub enum SerializedDataMessage {
    Ethereum {
        block: EthereumBlockData,
        transactions: Vec<EthereumTransactionData>,
        logs: Vec<Log>,
    },
}

impl SerializedDataMessage {
    pub fn get_block_ptr(&self) -> BlockPtr {
        match self {
            Self::Ethereum { block, .. } => BlockPtr {
                number: block.number.as_u64(),
                hash: block.hash.to_string(),
                parent_hash: block.parent_hash.to_string(),
            },
        }
    }
}

#[derive(Debug)]
pub struct EthereumFilteredEvent {
    pub datasource: String,
    pub handler: String,
    pub event: EthereumEventData,
}

#[derive(Debug)]
pub enum FilteredDataMessage {
    Ethereum {
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    },
}

impl FilteredDataMessage {
    pub fn get_block_ptr(&self) -> BlockPtr {
        match self {
            FilteredDataMessage::Ethereum { block, .. } => BlockPtr {
                number: block.number.as_u64(),
                hash: block.hash.to_string(),
                parent_hash: block.parent_hash.to_string(),
            },
        }
    }
}

pub type EntityType = String;
pub type EntityID = String;
pub type FieldName = String;
pub type RawEntity = HashMap<FieldName, Value>;

#[derive(Debug)]
pub enum StoreOperationMessage {
    Create((EntityType, RawEntity)),
    Load((EntityType, EntityID)),
    Update((EntityType, EntityID, RawEntity)),
    Delete((EntityType, EntityID)),
    LoadRelated((EntityType, EntityID, FieldName)),
    LoadInBlock((EntityType, EntityID)),
}

#[derive(Debug)]
pub enum StoreRequestResult {
    Create(String),
    Load(Option<RawEntity>),
    Delete,
    Update,
    LoadRelated(Vec<RawEntity>),
    LoadInBlock(Option<RawEntity>),
}
