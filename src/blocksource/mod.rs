mod readline;

use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceErr;
use crate::messages::SourceDataMessage;
use readline::Readline;
use tokio_stream::Stream;

pub enum BlockSource {
    Readline(Readline),
    ReadDir,
    Nats,
}

impl BlockSource {
    pub fn new(config: &Config) -> Result<Self, SourceErr> {
        let source = match config.source {
            SourceTypes::ReadLine => BlockSource::Readline(Readline()),
            _ => unimplemented!(),
        };
        Ok(source)
    }
}

pub async fn block_stream(
    source: BlockSource,
) -> Result<impl Stream<Item = SourceDataMessage>, SourceErr> {
    match source {
        BlockSource::Readline(readline) => Ok(readline.get_user_input_as_stream()),
        _ => unimplemented!(),
    }
}
