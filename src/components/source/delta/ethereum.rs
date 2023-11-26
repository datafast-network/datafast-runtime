use super::super::trino::ethereum::Header;
use super::super::trino::TrinoEthereumBlock;
use super::DeltaBlockTrait;
use crate::components::source::trino::ethereum::Log;
use crate::components::source::trino::ethereum::Transaction;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use deltalake::arrow::array::Array;
use deltalake::arrow::array::BooleanArray;
use deltalake::arrow::array::Int64Array;
use deltalake::arrow::array::ListArray;
use deltalake::arrow::array::StringArray;
use deltalake::arrow::array::StructArray;
use deltalake::arrow::record_batch::RecordBatch;

pub struct DeltaEthereumBlocks(Vec<TrinoEthereumBlock>);

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
            .map(|t| t.unwrap() as u64)
            .collect::<Vec<_>>();

        let log_types = value
            .column_by_name("log_type")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .into_iter()
            .map(|l| l.unwrap().to_owned())
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
                transaction_log_index: transaction_log_indexes.get(i).cloned(),
                log_type: log_types.get(i).cloned(),
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
            .downcast_ref::<StructArray>()
            .iter()
            .copied()
            .map(|s| {
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
            let block = TrinoEthereumBlock {
                chain_id,
                block_hash: block_hashes[i].clone(),
                parent_hash: parent_hashes[i].clone(),
                block_number: block_numbers[i],
                header: block_headers.0.get(i).cloned().unwrap(),
                transactions: block_transactions.get(i).cloned().unwrap().0,
                logs: block_logs.get(i).cloned().unwrap().0,
                created_at: created_at.get(i).cloned().unwrap(),
            };
            blocks.push(block);
        }

        Ok(Self(blocks))
    }
}

impl From<DeltaEthereumBlocks> for Vec<SerializedDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner.into_iter().map(SerializedDataMessage::from).collect()
    }
}

impl DeltaBlockTrait for DeltaEthereumBlocks {}
