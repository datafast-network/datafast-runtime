use super::base::BlockPtr;
use super::base::EntityID;
use super::base::EntityType;
use super::base::FieldName;
use super::base::RawEntity;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use web3::types::Log;

#[derive(Debug)]
pub enum BlockDataMessage {
    Ethereum {
        block: EthereumBlockData,
        transactions: Vec<EthereumTransactionData>,
        logs: Vec<Log>,
    },
}

impl BlockDataMessage {
    pub fn get_block_ptr(&self) -> BlockPtr {
        match self {
            Self::Ethereum { block, .. } => BlockPtr {
                number: block.number.as_u64(),
                hash: format!("{:?}", block.hash),
                parent_hash: format!("{:?}", block.parent_hash),
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
                hash: format!("{:?}", block.hash),
                parent_hash: format!("{:?}", block.parent_hash),
            },
        }
    }
}

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
