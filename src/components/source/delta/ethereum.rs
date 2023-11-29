use super::DeltaBlockTrait;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use deltalake::arrow::array::Array;
use deltalake::arrow::array::BooleanArray;
use deltalake::arrow::array::Int64Array;
use deltalake::arrow::array::ListArray;
use deltalake::arrow::array::StringArray;
use deltalake::arrow::array::StructArray;
use deltalake::arrow::record_batch::RecordBatch;
use ethabi::Bytes;
use hex::FromHex;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
use web3::types::Bytes as Web3Bytes;
use web3::types::Index;
use web3::types::Log as Web3Log;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;
use web3::types::U64;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DeltaEthereumBlock {
    pub chain_id: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub block_number: u64,
    pub header: Header,
    pub transactions: Vec<Transaction>,
    pub logs: Vec<Log>,
    pub created_at: u64,
}

impl From<&DeltaEthereumBlock> for EthereumBlockData {
    fn from(b: &DeltaEthereumBlock) -> Self {
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

impl From<&DeltaEthereumBlock> for Vec<EthereumTransactionData> {
    fn from(value: &DeltaEthereumBlock) -> Self {
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
                input: Bytes::from_hex(&tx.input.replace("0x", "")).unwrap_or_default(),
                nonce: U256::from(tx.nonce),
            };
            result.push(tx_data);
        }

        result
    }
}

impl From<&DeltaEthereumBlock> for Vec<Web3Log> {
    fn from(b: &DeltaEthereumBlock) -> Self {
        let mut result = vec![];

        for log in b.logs.iter() {
            let log_data = Web3Log {
                address: H160::from_str(&log.address).unwrap(),
                topics: log
                    .topics
                    .iter()
                    .map(|t| H256::from_str(t).unwrap())
                    .collect(),
                data: Web3Bytes::from(
                    Bytes::from_hex(log.data.replace("0x", "")).unwrap_or_default(),
                ),
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

impl From<DeltaEthereumBlock> for SerializedDataMessage {
    fn from(value: DeltaEthereumBlock) -> Self {
        SerializedDataMessage::Ethereum {
            block: EthereumBlockData::from(&value),
            transactions: Vec::<EthereumTransactionData>::from(&value),
            logs: Vec::<Web3Log>::from(&value),
        }
    }
}

pub struct DeltaEthereumBlocks(Vec<DeltaEthereumBlock>);

#[derive(Debug)]
pub struct DeltaEthereumHeaders(Vec<Header>);

#[derive(Debug, Clone, Default)]
pub struct DeltaEthereumTransactions(Vec<Transaction>);

#[derive(Debug, Clone)]
pub struct DeltaEthereumLogs(Vec<Log>);

impl TryFrom<&StructArray> for DeltaEthereumHeaders {
    type Error = SourceError;
    fn try_from(value: &StructArray) -> Result<Self, Self::Error> {
        let authors = value
            .column_by_name("author")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let state_roots = value
            .column_by_name("state_root")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let transaction_roots = value
            .column_by_name("transactions_root")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let receipt_roots = value
            .column_by_name("receipts_root")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let gas_useds = value
            .column_by_name("gas_used")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let gas_limits = value
            .column_by_name("gas_limit")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let extra_datas = value
            .column_by_name("extra_data")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let logs_blooms = value
            .column_by_name("logs_bloom")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let timestamps = value
            .column_by_name("timestamp")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|t| t.unwrap().to_owned())
            .collect::<Vec<_>>();

        let difficulties = value
            .column_by_name("difficulty")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|d| d.unwrap().to_owned())
            .collect::<Vec<_>>();

        let seal_fields = value
            .column_by_name("seal_fields")
            .unwrap()
            .as_any()
            .downcast_ref::<ListArray>()
            .unwrap()
            .iter()
            .map(|s| {
                s.unwrap()
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .into_iter()
                    .map(|s| s.unwrap().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let sizes = value
            .column_by_name("size")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|s| s.unwrap() as u64)
            .collect::<Vec<_>>();

        let base_fee_per_gass = value
            .column_by_name("base_fee_per_gas")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|b| b.map(|s| s.to_owned()))
            .collect::<Vec<_>>();

        let nonces = value
            .column_by_name("nonce")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|n| n.unwrap().to_owned())
            .collect::<Vec<_>>();

        let number_rows = authors.len();
        let mut headers = vec![];

        for i in 0..number_rows {
            let header = Header {
                author: authors[i].clone(),
                state_root: state_roots[i].clone(),
                transactions_root: transaction_roots[i].clone(),
                receipts_root: receipt_roots[i].clone(),
                gas_used: gas_useds[i].clone(),
                gas_limit: gas_limits[i].clone(),
                extra_data: extra_datas[i].clone(),
                logs_bloom: logs_blooms.get(i).cloned(),
                timestamp: timestamps[i].clone(),
                difficulty: difficulties[i].clone(),
                total_difficulty: String::default(),
                seal_fields: seal_fields[i].clone(),
                size: sizes.get(i).copied(),
                base_fee_per_gas: base_fee_per_gass.get(i).cloned().unwrap(),
                nonce: nonces[i].clone(),
            };
            headers.push(header);
        }

        Ok(Self(headers))
    }
}

impl TryFrom<&StructArray> for DeltaEthereumTransactions {
    type Error = SourceError;
    fn try_from(value: &StructArray) -> Result<Self, Self::Error> {
        let hashes = value
            .column_by_name("hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let nonces = value
            .column_by_name("nonce")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|n| n.unwrap() as u64)
            .collect::<Vec<_>>();

        let block_hashes = value
            .column_by_name("block_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|b| b.unwrap().to_owned())
            .collect::<Vec<_>>();

        let block_numbers = value
            .column_by_name("block_number")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|b| b.unwrap() as u64)
            .collect::<Vec<_>>();

        let transaction_indexes = value
            .column_by_name("transaction_index")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|t| t.unwrap() as u64)
            .collect::<Vec<_>>();

        let from_addresses = value
            .column_by_name("from_address")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|f| f.unwrap().to_owned())
            .collect::<Vec<_>>();

        let to_addresses = value
            .column_by_name("to_address")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|t| t.map(|s| s.to_owned()))
            .collect::<Vec<_>>();

        let values = value
            .column_by_name("value")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap().to_owned())
            .collect::<Vec<_>>();

        let gas_prices = value
            .column_by_name("gas_price")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|g| g.unwrap().to_owned())
            .collect::<Vec<_>>();

        let gas = value
            .column_by_name("gas")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|g| g.unwrap().to_owned())
            .collect::<Vec<_>>();

        let inputs = value
            .column_by_name("input")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|i| i.unwrap().to_owned())
            .collect::<Vec<_>>();

        let v_values = value
            .column_by_name("v")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|v| v.unwrap() as u64)
            .collect::<Vec<_>>();

        let r_values = value
            .column_by_name("r")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|r| r.unwrap().to_owned())
            .collect::<Vec<_>>();

        let s_values = value
            .column_by_name("s")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|s| s.unwrap().to_owned())
            .collect::<Vec<_>>();

        let num_rows = hashes.len();
        let mut transactions = vec![];

        for i in 0..num_rows {
            let transaction = Transaction {
                hash: hashes[i].clone(),
                nonce: nonces[i],
                block_hash: block_hashes.get(i).cloned(),
                block_number: block_numbers.get(i).cloned(),
                transaction_index: transaction_indexes.get(i).cloned(),
                from_address: from_addresses[i].clone(),
                to_address: to_addresses.get(i).cloned().unwrap(),
                value: values[i].clone(),
                gas_price: gas_prices.get(i).cloned(),
                gas: gas[i].clone(),
                input: inputs[i].clone(),
                v: v_values[i],
                r: r_values[i].clone(),
                s: s_values[i].clone(),
                transaction_type: None,
                access_list: None,
                max_priority_fee_per_gas: None,
                max_fee_per_gas: None,
            };
            transactions.push(transaction);
        }
        Ok(Self(transactions))
    }
}

impl TryFrom<&StructArray> for DeltaEthereumLogs {
    type Error = SourceError;
    fn try_from(value: &StructArray) -> Result<Self, Self::Error> {
        let addresses = value
            .column_by_name("address")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|a| a.unwrap().to_owned())
            .collect::<Vec<_>>();

        let topics = value
            .column_by_name("topics")
            .unwrap()
            .as_any()
            .downcast_ref::<ListArray>()
            .unwrap()
            .iter()
            .map(|t| {
                t.unwrap()
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.unwrap().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let data = value
            .column_by_name("data")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|d| d.unwrap().to_owned())
            .collect::<Vec<_>>();

        let block_hashes = value
            .column_by_name("block_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|b| b.unwrap().to_owned())
            .collect::<Vec<_>>();

        let block_numbers = value
            .column_by_name("block_number")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|b| b.unwrap() as u64)
            .collect::<Vec<_>>();

        let transaction_hashes = value
            .column_by_name("transaction_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|t| t.unwrap().to_owned())
            .collect::<Vec<_>>();

        let transaction_indexes = value
            .column_by_name("transaction_index")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|t| t.unwrap() as u64)
            .collect::<Vec<_>>();

        let log_indexes = value
            .column_by_name("log_index")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|l| l.unwrap() as u64)
            .collect::<Vec<_>>();

        let transaction_log_indexes = value
            .column_by_name("transaction_log_index")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|t| t.map(|v| v as u64))
            .collect::<Vec<_>>();

        let log_types = value
            .column_by_name("log_type")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|l| l.map(|v| v.to_owned()))
            .collect::<Vec<_>>();

        let removeds = value
            .column_by_name("removed")
            .unwrap()
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap()
            .into_iter()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();

        let num_rows = addresses.len();
        let mut logs = vec![];

        for i in 0..num_rows {
            let log = Log {
                address: addresses[i].clone(),
                topics: topics[i].clone(),
                data: data[i].clone(),
                block_hash: block_hashes.get(i).cloned(),
                block_number: block_numbers.get(i).cloned(),
                transaction_hash: transaction_hashes.get(i).cloned(),
                transaction_index: transaction_indexes.get(i).cloned(),
                log_index: log_indexes.get(i).cloned(),
                transaction_log_index: transaction_log_indexes.get(i).cloned().unwrap(),
                log_type: log_types.get(i).cloned().unwrap(),
                removed: removeds.get(i).cloned(),
            };
            logs.push(log);
        }
        Ok(Self(logs))
    }
}

impl TryFrom<RecordBatch> for DeltaEthereumBlocks {
    type Error = SourceError;
    fn try_from(value: RecordBatch) -> Result<Self, Self::Error> {
        let chain_id = value
            .column_by_name("chain_id")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0) as u64;

        let block_hashes = value
            .column_by_name("block_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let parent_hashes = value
            .column_by_name("parent_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let block_numbers = value
            .column_by_name("block_number")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|h| h.unwrap() as u64)
            .collect::<Vec<_>>();

        let block_headers = value
            .column_by_name("header")
            .unwrap()
            .as_any()
            .downcast_ref::<StructArray>()
            .to_owned()
            .map(|s| DeltaEthereumHeaders::try_from(s).unwrap())
            .unwrap();

        let block_transactions = value
            .column_by_name("transactions")
            .unwrap()
            .as_any()
            .downcast_ref::<ListArray>()
            .unwrap()
            .iter()
            .map(|s| {
                let s = s.unwrap();
                let structs = s.as_any().downcast_ref::<StructArray>().unwrap();
                DeltaEthereumTransactions::try_from(structs).unwrap()
            })
            .collect::<Vec<_>>();

        let block_logs = value
            .column_by_name("logs")
            .unwrap()
            .as_any()
            .downcast_ref::<ListArray>()
            .unwrap()
            .iter()
            .map(|s| {
                let s = s.unwrap();
                let structs = s.as_any().downcast_ref::<StructArray>().unwrap();
                DeltaEthereumLogs::try_from(structs).unwrap()
            })
            .collect::<Vec<_>>();

        let created_at = value
            .column_by_name("created_at")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .into_iter()
            .map(|c| c.unwrap() as u64)
            .collect::<Vec<_>>();

        let num_rows = block_numbers.len();
        let mut blocks = vec![];

        for i in 0..num_rows {
            let block = DeltaEthereumBlock {
                chain_id,
                block_hash: block_hashes[i].clone(),
                parent_hash: parent_hashes[i].clone(),
                block_number: block_numbers[i],
                header: block_headers.0.get(i).cloned().unwrap(),
                transactions: block_transactions.get(i).cloned().unwrap().0,
                logs: block_logs.get(i).cloned().unwrap().0,
                created_at: created_at[i],
            };
            blocks.push(block);
        }

        Ok(Self(blocks))
    }
}

impl From<DeltaEthereumBlocks> for Vec<SerializedDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner
            .into_par_iter()
            .map(SerializedDataMessage::from)
            .collect()
    }
}

impl DeltaBlockTrait for DeltaEthereumBlocks {}
