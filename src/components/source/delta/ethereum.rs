use super::super::trino::TrinoEthereumBlock;
use super::DeltaBlockTrait;
use crate::errors::SourceError;
use crate::info;
use crate::messages::SerializedDataMessage;
use deltalake::arrow::array::{Int64Array, StringArray};
use deltalake::arrow::record_batch::RecordBatch;

pub struct DeltaEthereumBlocks(Vec<TrinoEthereumBlock>);

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
            .iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let parent_hashes = value
            .column_by_name("parent_hash")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .iter()
            .map(|h| h.unwrap().to_owned())
            .collect::<Vec<_>>();

        let block_numbers = value
            .column_by_name("block_number")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .iter()
            .map(|h| h.unwrap() as u64)
            .collect::<Vec<_>>();

        info!(
            DeltaEthereumBlocks,
            "serialize RecordBatch[] to blocks[]";
            chain_id => chain_id,
            block_numbers => format!("{:?}", block_numbers)
        );

        Ok(Self(vec![]))
    }
}

impl From<DeltaEthereumBlocks> for Vec<SerializedDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner.into_iter().map(SerializedDataMessage::from).collect()
    }
}

impl DeltaBlockTrait for DeltaEthereumBlocks {}
