use super::super::trino::TrinoEthereumBlock;
use super::DeltaBlockTrait;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use deltalake::arrow::record_batch::RecordBatch;

pub struct DeltaEthereumBlocks(Vec<TrinoEthereumBlock>);

impl TryFrom<Vec<RecordBatch>> for DeltaEthereumBlocks {
    type Error = SourceError;
    fn try_from(value: Vec<RecordBatch>) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl From<DeltaEthereumBlocks> for Vec<SerializedDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner.into_iter().map(SerializedDataMessage::from).collect()
    }
}

impl DeltaBlockTrait for DeltaEthereumBlocks {}
