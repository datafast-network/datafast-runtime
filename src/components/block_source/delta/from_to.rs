use crate::common::BlockDataMessage;
use crate::components::block_source::delta::DeltaBlockTrait;
use crate::components::block_source::proto::ethereum::Block as PbBlock;
use deltalake::arrow::array::BinaryArray;
use deltalake::arrow::record_batch::RecordBatch;
use prost::Message;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;

use crate::errors::SourceError;

pub struct DeltaEthereumBlocks(Vec<PbBlock>);

impl DeltaBlockTrait for DeltaEthereumBlocks {}

impl TryFrom<RecordBatch> for DeltaEthereumBlocks {
    type Error = SourceError;
    fn try_from(value: RecordBatch) -> Result<Self, Self::Error> {
        let block_data = value
            .column_by_name("block_data")
            .unwrap()
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap();

        let blocks = block_data
            .into_iter()
            .map(|b| PbBlock::decode(&mut b.unwrap()).unwrap())
            .collect::<Vec<PbBlock>>();

        Ok(Self(blocks))
    }
}

impl From<DeltaEthereumBlocks> for Vec<BlockDataMessage> {
    fn from(value: DeltaEthereumBlocks) -> Self {
        let inner = value.0;
        inner.into_par_iter().map(BlockDataMessage::from).collect()
    }
}
