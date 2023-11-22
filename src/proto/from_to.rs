use super::ethereum::Block;
use super::ethereum::Log;
use super::ethereum::Transaction;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use anyhow::anyhow;
use std::str::FromStr;
use web3::types::H160;
use web3::types::H256;
use web3::types::U256;

impl TryFrom<Block> for EthereumBlockData {
    type Error = anyhow::Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let header = value.header.ok_or(anyhow!("block header is none"))?;
        let block_data = EthereumBlockData {
            number: value.block_number.into(),
            gas_used: U256::from_str(header.gas_used.as_str())?,
            gas_limit: U256::from_str(header.gas_limit.as_str())?,
            timestamp: U256::from_str(header.timestamp.as_str())?,
            difficulty: U256::from_str(header.difficulty.as_str())?,
            total_difficulty: U256::from_str(header.total_difficulty.as_str())?,
            size: header.size.map(|size| size.into()),
            hash: H256::from_str(value.block_hash.as_str())?,
            parent_hash: H256::from_str(value.parent_hash.as_str())?,
            uncles_hash: H256::default(),
            author: H160::from_str(header.author.as_str())?,
            state_root: H256::from_str(header.state_root.as_str())?,
            transactions_root: H256::from_str(header.transactions_root.as_str())?,
            receipts_root: H256::from_str(header.receipts_root.as_str())?,
            base_fee_per_gas: header
                .base_fee_per_gas
                .map(|fee| U256::from_str(fee.as_str()))
                .transpose()?,
        };

        Ok(block_data)
    }
}

impl TryFrom<Transaction> for EthereumTransactionData {
    type Error = anyhow::Error;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        let transaction_data = EthereumTransactionData {
            hash: H256::from_str(value.hash.as_str())?,
            index: value
                .transaction_index
                .map(|index| index.into())
                .unwrap_or_default(),
            from: H160::from_str(value.from_address.as_str())?,
            to: value
                .to_address
                .map(|to| H160::from_str(to.as_str()))
                .transpose()?,
            value: U256::from_str(value.value.as_str())?,
            gas_price: value
                .gas_price
                .map(|price| U256::from_str(price.as_str()))
                .transpose()
                .unwrap()
                .unwrap(),
            input: value.input.into_bytes(),
            nonce: value.nonce.into(),
            gas_limit: U256::from_str(value.gas.as_str())?,
        };

        Ok(transaction_data)
    }
}

impl TryFrom<Log> for web3::types::Log {
    type Error = anyhow::Error;

    fn try_from(value: Log) -> Result<Self, Self::Error> {
        let log = web3::types::Log {
            address: H160::from_str(value.address.as_str())?,
            topics: value
                .topics
                .into_iter()
                .map(|topic| H256::from_str(topic.as_str()))
                .collect::<Result<Vec<H256>, _>>()?,
            data: web3::types::Bytes(value.data.into_bytes()),
            block_hash: value
                .block_hash
                .map(|hash| H256::from_str(hash.as_str()))
                .transpose()?,
            block_number: value.block_number.map(|number| number.into()),
            transaction_hash: value
                .transaction_hash
                .map(|hash| H256::from_str(hash.as_str()))
                .transpose()?,
            transaction_index: value.transaction_index.map(|index| index.into()),
            log_index: value.log_index.map(|index| index.into()),
            transaction_log_index: None,
            log_type: value.log_type,
            removed: value.removed,
        };

        Ok(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    #[test]
    fn test_from_to_eth_block() {
        let block_file = File::open("./src/tests/blocks/block.json").unwrap();
        let block: Block = serde_json::from_reader(block_file).unwrap();
        let block_data = EthereumBlockData::try_from(block.clone()).unwrap();
        assert_eq!(block_data.number, 10000000.into());
        assert_eq!(
            block_data.hash,
            H256::from_str(block.clone().block_hash.as_str()).unwrap()
        );
        assert_eq!(
            block_data.parent_hash,
            H256::from_str(block.clone().parent_hash.as_str()).unwrap()
        );
        let txs = block
            .clone()
            .transactions
            .into_iter()
            .map(EthereumTransactionData::try_from)
            .collect::<Result<Vec<EthereumTransactionData>, _>>()
            .unwrap();

        assert_eq!(txs.len(), 2);
        let logs = block
            .clone()
            .logs
            .into_iter()
            .map(web3::types::Log::try_from)
            .collect::<Result<Vec<web3::types::Log>, _>>()
            .unwrap();

        assert_eq!(logs.len(), 2);
    }
}
