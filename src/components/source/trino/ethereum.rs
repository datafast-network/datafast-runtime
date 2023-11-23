use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use prusto_rs::Row;
use serde::Deserialize;
use serde::Serialize;
use web3::types::Log;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrinoEthereumBlock {
    pub chain_id: u64,
    pub block_hash: String,
    pub parent_hash: String,
    pub block_number: u64,
    pub header: Option<EthereumBlockData>,
    pub transactions: Vec<EthereumTransactionData>,
    pub logs: Vec<Log>,
}

impl Into<SerializedDataMessage> for TrinoEthereumBlock {
    fn into(self) -> SerializedDataMessage {
        SerializedDataMessage::Ethereum {
            block: self.header.unwrap(),
            transactions: self.transactions,
            logs: self.logs,
        }
    }
}

impl TryFrom<Row> for TrinoEthereumBlock {
    type Error = SourceError;
    fn try_from(value: Row) -> Result<Self, Self::Error> {
        let row_json = value.into_json();
        let mut object = serde_json::Map::new();
        for (idx, field) in [
            "chain_id",
            "block_hash",
            "parent_hash",
            "block_number",
            "header",
            "transactions",
            "logs",
        ]
        .into_iter()
        .enumerate()
        {
            object.insert(
                field.to_owned(),
                row_json.get(idx).cloned().to_owned().unwrap(),
            );
        }
        let this: Self = serde_json::from_value(serde_json::Value::Object(object)).unwrap();
        Ok(this)
    }
}
