use df_types::chain::ethereum::block::EthereumBlockData;
use df_types::chain::ethereum::event::EthereumEventData;
use df_types::chain::ethereum::transaction::EthereumTransactionData;
use df_types::chain::ethereum::transaction::EthereumTransactionReceipt;
use df_types::web3::types::Log;
use crate::common::{BlockPtr, EntityID, EntityType, FieldName, RawEntity};

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
                hash: format!("{:?}", block.hash).to_lowercase(),
                parent_hash: format!("{:?}", block.parent_hash).to_lowercase(),
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
        txs: Vec<EthereumTransactionReceipt>,
    },
}

impl FilteredDataMessage {
    pub fn get_block_ptr(&self) -> BlockPtr {
        match self {
            FilteredDataMessage::Ethereum { block, .. } => BlockPtr {
                number: block.number.as_u64(),
                hash: format!("{:?}", block.hash).to_lowercase(),
                parent_hash: format!("{:?}", block.parent_hash).to_lowercase(),
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

impl StoreOperationMessage {
    pub fn operation_type(&self) -> String {
        match self {
            Self::Create(_) => "CREATE".to_owned(),
            Self::Load(_) => "LOAD".to_owned(),
            Self::Update(_) => "UPDATE".to_owned(),
            Self::Delete(_) => "DELETE".to_owned(),
            Self::LoadRelated(_) => "LOAD_RELATED".to_owned(),
            Self::LoadInBlock(_) => "LOAD_IN_BLOCK".to_owned(),
        }
    }

    pub fn entity_type(&self) -> String {
        match self {
            Self::Create((entity, _)) => entity.to_owned(),
            Self::Load((entity, _)) => entity.to_owned(),
            Self::Update((entity, ..)) => entity.to_owned(),
            Self::Delete((entity, _)) => entity.to_owned(),
            Self::LoadRelated((entity, ..)) => entity.to_owned(),
            Self::LoadInBlock((entity, _)) => entity.to_owned(),
        }
    }
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
