use super::ethereum;
use df_types::chain::ethereum::block::EthereumBlockData;
use df_types::chain::ethereum::transaction::EthereumTransactionData;
use crate::common::BlockDataMessage;
use ethabi::ethereum_types::H160;
use ethabi::ethereum_types::H256;
use ethabi::ethereum_types::U128;
use ethabi::ethereum_types::U256;
use ethabi::ethereum_types::U64;
use ethabi::Bytes;
use hex::FromHex;
use std::str::FromStr;
use df_types::web3::types::Bytes as Web3Bytes;
use df_types::web3::types::Index;
use df_types::web3::types::Log as Web3Log;

impl From<&ethereum::Block> for EthereumBlockData {
    fn from(b: &ethereum::Block) -> Self {
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
            size: header.size.map(U256::from),
            base_fee_per_gas: header
                .base_fee_per_gas
                .map(|s| U256::from_dec_str(&s).unwrap()),
        }
    }
}

impl From<&ethereum::Transaction> for EthereumTransactionData {
    fn from(tx: &ethereum::Transaction) -> Self {
        EthereumTransactionData {
            hash: H256::from_str(&tx.hash).unwrap(),
            index: U128::from(tx.transaction_index.unwrap() as i64),
            from: H160::from_str(&tx.from_address).unwrap(),
            to: tx.to_address.clone().map(|a| H160::from_str(&a).unwrap()),
            value: U256::from_dec_str(&tx.value).unwrap(),
            gas_limit: U256::from_dec_str(&tx.gas).unwrap(),
            gas_price: U256::from_dec_str(&tx.gas_price.clone().unwrap_or_default()).unwrap(),
            input: Bytes::from_hex(tx.input.replace("0x", "")).unwrap_or_default(),
            nonce: U256::from(tx.nonce),
        }
    }
}

impl From<&ethereum::Block> for Vec<Web3Log> {
    fn from(b: &ethereum::Block) -> Self {
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

impl From<ethereum::Block> for BlockDataMessage {
    fn from(block: ethereum::Block) -> Self {
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
