use super::proto::ethereum::Block as PbBlock;
use super::proto::ethereum::Transaction as PbTransaction;
use super::DeltaBlockTrait;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::errors::SourceError;
use crate::info;
use crate::messages::BlockDataMessage;
use deltalake::arrow::array::Array;
use deltalake::arrow::array::BinaryArray;
use deltalake::arrow::record_batch::RecordBatch;
use ethabi::Bytes;
use hex::FromHex;
use prost::Message;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use std::str::FromStr;
use web3::types::Bytes as Web3Bytes;
use web3::types::Index;
use web3::types::Log as Web3Log;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;
use web3::types::U64;

impl From<&PbBlock> for EthereumBlockData {
    fn from(b: &PbBlock) -> Self {
        let header = b.header.clone().unwrap();
        EthereumBlockData {
            hash: H256::from_str(&b.block_hash).unwrap(),
            parent_hash: H256::from_str(&b.parent_hash).unwrap(),
            uncles_hash: H256::default(),
            author: H160::from_str(&header.author).unwrap(),
            state_root: H256::from_str(&header.state_root).unwrap(),
            transactions_root: H256::from_str(&header.transactions_root).unwrap(),
            receipts_root: H256::from_str(&header.receipts_root).unwrap(),
            number: U64::from(b.block_number),
            gas_used: U256::from_dec_str(&header.gas_used).unwrap(),
            gas_limit: U256::from_dec_str(&header.gas_limit).unwrap(),
            timestamp: U256::from_dec_str(&header.timestamp).unwrap(),
            difficulty: U256::from_dec_str(&header.difficulty).unwrap(),
            total_difficulty: U256::from_dec_str(&header.total_difficulty).unwrap(),
            size: header.size.map(|s| U256::from(s)),
            base_fee_per_gas: header
                .base_fee_per_gas
                .map(|s| U256::from_dec_str(&s).unwrap()),
        }
    }
}

impl From<&PbTransaction> for EthereumTransactionData {
    fn from(tx: &PbTransaction) -> Self {
        EthereumTransactionData {
            hash: H256::from_str(&tx.hash).unwrap(),
            index: U128::from(tx.transaction_index.unwrap() as i64),
            from: H160::from_str(&tx.from_address).unwrap(),
            to: tx.to_address.clone().map(|a| H160::from_str(&a).unwrap()),
            value: U256::from_dec_str(&tx.value).unwrap(),
            gas_limit: U256::from_dec_str(&tx.gas).unwrap(),
            gas_price: U256::from_dec_str(&tx.gas_price.clone().unwrap_or_default()).unwrap(),
            input: Bytes::from_hex(&tx.input.replace("0x", "")).unwrap_or_default(),
            nonce: U256::from(tx.nonce),
        }
    }
}

impl From<&PbBlock> for Vec<Web3Log> {
    fn from(b: &PbBlock) -> Self {
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

pub struct DeltaEthereumBlocks(Vec<PbBlock>);

impl From<PbBlock> for BlockDataMessage {
    fn from(block: PbBlock) -> Self {
        BlockDataMessage::Ethereum {
            block: EthereumBlockData::from(&block),
            transactions: block
                .transactions
                .iter()
                .map(EthereumTransactionData::from)
                .collect(),
            logs: Vec::<Web3Log>::from(&block),
        }
    }
}

impl TryFrom<RecordBatch> for DeltaEthereumBlocks {
    type Error = SourceError;
    fn try_from(value: RecordBatch) -> Result<Self, Self::Error> {
        let block_data = value
            .column_by_name("block_data")
            .unwrap()
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();

        log::info!("------> Downcast OK");

        let blocks = block_data
            .into_iter()
            .map(|b| PbBlock::decode(b.unwrap()).unwrap())
            .collect::<Vec<PbBlock>>();
        log::info!("------> serialized OK");
        Ok(Self(blocks))
    }
}

impl From<DeltaEthereumBlocks> for Vec<BlockDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner.into_par_iter().map(BlockDataMessage::from).collect()
    }
}

impl DeltaBlockTrait for DeltaEthereumBlocks {}
