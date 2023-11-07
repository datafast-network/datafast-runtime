use crate::config::SourceTypes;
use crate::{config::Config, errors::SourceErr, messages::SourceDataMessage};
use kanal::AsyncSender;

pub enum BlockSource {
    Readline,
    ReadDir,
    Nats,
}

impl BlockSource {
    pub async fn new(cfg: &Config) -> Result<Self, SourceErr> {
        match cfg.source {
            SourceTypes::ReadLine => Self::Readline,
            SourceTypes::ReadDir { source_dir } => Self::Readline,
            SourceTypes::Nats { uri, subject } => Self::Nats,
        }
    }

    pub async fn run(self, input_sender: AsyncSender<SourceDataMessage>) -> Result<(), SourceErr> {
        todo!()
    }
}
