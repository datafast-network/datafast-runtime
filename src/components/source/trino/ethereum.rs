use super::utils::from_vec_json_value;
use super::TrinoBlockTrait;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use ethabi::Bytes;
use hex::FromHex;
use prusto::Row;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::str::FromStr;
use web3::types::Bytes as Web3Bytes;
use web3::types::Index;
use web3::types::Log as Web3Log;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;
use web3::types::U64;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Header {
    pub author: String,
    pub state_root: String,
    pub transactions_root: String,
    pub receipts_root: String,
    pub gas_used: String,
    pub gas_limit: String,
    pub extra_data: String,
    pub logs_bloom: Option<String>,
    pub timestamp: String,
    pub difficulty: String,
    pub total_difficulty: String,
    pub seal_fields: Vec<String>,
    pub size: Option<u64>,
    pub base_fee_per_gas: Option<String>,
    pub nonce: String,
}

from_vec_json_value!(
    Header;
    author => String,
    state_root => String,
    transactions_root => String,
    receipts_root => String,
    gas_used => String,
    gas_limit => String,
    extra_data => String,
    logs_bloom => Option<String>,
    timestamp => String,
    difficulty => String,
    total_difficulty => String,
    seal_fields => Vec<String>,
    size => Option<u64>,
    base_fee_per_gas => Option<String>,
    nonce => String
);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Transaction {
    pub hash: String,
    pub nonce: u64,
    pub block_hash: Option<String>,
    pub block_number: Option<u64>,
    pub transaction_index: Option<u64>,
    pub from_address: String,
    pub to_address: Option<String>,
    pub value: String,
    pub gas_price: Option<String>,
    pub gas: String,
    pub input: String,
    pub v: u64,
    pub r: String,
    pub s: String,
    pub transaction_type: Option<i32>,
    pub access_list: Option<String>,
    pub max_priority_fee_per_gas: Option<String>,
    pub max_fee_per_gas: Option<String>,
}

from_vec_json_value!(
    Transaction;
    hash => String,
    nonce => u64,
    block_hash => Option<String>,
    block_number => Option<u64>,
    transaction_index => Option<u64>,
    from_address => String,
    to_address => Option<String>,
    value => String,
    gas_price => Option<String>,
    gas => String,
    input => String,
    v => u64,
    r => String,
    s => String,
    transaction_type => Option<i32>,
    access_list => Option<String>,
    max_priority_fee_per_gas => Option<String>,
    max_fee_per_gas => Option<String>
);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_hash: Option<String>,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub transaction_index: Option<u64>,
    pub log_index: Option<u64>,
    pub transaction_log_index: Option<u64>,
    pub log_type: Option<String>,
    pub removed: Option<bool>,
}

from_vec_json_value!(
    Log;
    address => String,
    topics => Vec<String>,
    data => String,
    block_hash => Option<String>,
    block_number => Option<u64>,
    transaction_hash => Option<String>,
    transaction_index => Option<u64>,
    log_index => Option<u64>,
    transaction_log_index => Option<u64>,
    log_type => Option<String>,
    removed => Option<bool>
);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TrinoEthereumBlock {
    pub chain_id: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub block_number: u64,
    pub header: Header,
    pub transactions: Vec<Transaction>,
    pub logs: Vec<Log>,
    pub created_at: u64,
}

impl TryFrom<Vec<Value>> for TrinoEthereumBlock {
    type Error = SourceError;
    fn try_from(values: Vec<Value>) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: serde_json::from_value(values.get(0).cloned().unwrap()).unwrap(),
            block_hash: serde_json::from_value(values.get(1).cloned().unwrap()).unwrap(),
            parent_hash: serde_json::from_value(values.get(2).cloned().unwrap()).unwrap(),
            block_number: serde_json::from_value(values.get(3).cloned().unwrap()).unwrap(),
            header: Header::try_from(values.get(4).cloned().unwrap().as_array().cloned().unwrap())?,
            transactions: values
                .get(5)
                .cloned()
                .unwrap()
                .as_array()
                .cloned()
                .unwrap()
                .into_iter()
                .flat_map(|v| Transaction::try_from(v.as_array().cloned().unwrap()))
                .collect(),
            logs: values
                .get(6)
                .cloned()
                .unwrap()
                .as_array()
                .cloned()
                .unwrap()
                .into_iter()
                .flat_map(|v| Log::try_from(v.as_array().cloned().unwrap()))
                .collect(),
            created_at: serde_json::from_value(values.get(7).cloned().unwrap()).unwrap(),
        })
    }
}

impl TryFrom<Row> for TrinoEthereumBlock {
    type Error = SourceError;

    fn try_from(value: Row) -> Result<Self, Self::Error> {
        let jsons = value.into_json();
        Self::try_from(jsons)
    }
}

impl From<&TrinoEthereumBlock> for EthereumBlockData {
    fn from(b: &TrinoEthereumBlock) -> Self {
        Self {
            hash: H256::from_str(&b.block_hash).unwrap(),
            parent_hash: H256::from_str(&b.parent_hash).unwrap(),
            uncles_hash: H256::default(),
            author: H160::from_str(&b.header.author).unwrap(),
            state_root: H256::from_str(&b.header.state_root).unwrap(),
            transactions_root: H256::from_str(&b.header.transactions_root).unwrap(),
            receipts_root: H256::from_str(&b.header.receipts_root).unwrap(),
            number: U64::from(b.block_number),
            gas_used: U256::from_dec_str(&b.header.gas_used).unwrap(),
            gas_limit: U256::from_dec_str(&b.header.gas_limit).unwrap(),
            timestamp: U256::from_dec_str(&b.header.timestamp).unwrap(),
            difficulty: U256::from_dec_str(&b.header.difficulty).unwrap(),
            total_difficulty: U256::from_dec_str(&b.header.total_difficulty).unwrap(),
            size: None,
            base_fee_per_gas: None,
        }
    }
}

impl From<&TrinoEthereumBlock> for Vec<EthereumTransactionData> {
    fn from(value: &TrinoEthereumBlock) -> Self {
        let mut result = vec![];

        for tx in value.transactions.iter() {
            let tx_data = EthereumTransactionData {
                hash: H256::from_str(&tx.hash).unwrap(),
                index: U128::from(tx.transaction_index.unwrap()),
                from: H160::from_str(&tx.from_address).unwrap(),
                to: tx
                    .to_address
                    .clone()
                    .map(|addr| H160::from_str(&addr).unwrap()),
                value: U256::from_dec_str(&tx.value).unwrap(),
                gas_limit: U256::from_dec_str(&tx.gas).unwrap(),
                gas_price: U256::from_dec_str(&tx.gas_price.clone().unwrap_or_default())
                    .unwrap_or_default(),
                input: Bytes::from_hex(&tx.input).unwrap_or_default(),
                nonce: U256::from(tx.nonce),
            };
            result.push(tx_data);
        }

        result
    }
}

impl From<&TrinoEthereumBlock> for Vec<Web3Log> {
    fn from(b: &TrinoEthereumBlock) -> Self {
        let mut result = vec![];

        for log in b.logs.iter() {
            let log_data = Web3Log {
                address: H160::from_str(&log.address).unwrap(),
                topics: log
                    .topics
                    .iter()
                    .map(|t| H256::from_str(t).unwrap())
                    .collect(),
                data: Web3Bytes::from(Bytes::from_hex(log.data.clone()).unwrap_or_default()),
                block_hash: Some(H256::from_str(&b.block_hash).unwrap()),
                block_number: Some(U64::from(b.block_number)),
                transaction_hash: Some(
                    H256::from_str(&log.transaction_hash.clone().unwrap()).unwrap(),
                ),
                transaction_index: Some(Index::from(log.transaction_index.unwrap())),
                log_index: Some(U256::from(log.log_index.unwrap())),
                transaction_log_index: log.transaction_log_index.map(U256::from),
                log_type: log.log_type.clone(),
                removed: log.removed,
            };
            result.push(log_data);
        }

        result
    }
}

impl From<TrinoEthereumBlock> for SerializedDataMessage {
    fn from(value: TrinoEthereumBlock) -> Self {
        SerializedDataMessage::Ethereum {
            block: EthereumBlockData::from(&value),
            transactions: Vec::<EthereumTransactionData>::from(&value),
            logs: Vec::<Web3Log>::from(&value),
        }
    }
}

impl TrinoBlockTrait for TrinoEthereumBlock {
    fn get_block_hash(&self) -> String {
        self.block_hash.clone()
    }

    fn get_parent_hash(&self) -> String {
        self.parent_hash.clone()
    }

    fn get_block_number(&self) -> u64 {
        self.block_number
    }

    fn get_insert_timestamp(&self) -> u64 {
        self.created_at
    }
}
