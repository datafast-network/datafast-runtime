mod nats;
mod readdir;
mod readline;

use crate::components::source::nats::NatsConsumer;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use crate::messages::EthereumFullBlock;
use crate::messages::SourceDataMessage;
use crate::proto::ethereum::Block;
use crate::proto::ethereum::Blocks;
use prost::Message;

use futures_util::pin_mut;
use kanal::AsyncSender;
use readdir::ReadDir;
use readline::Readline;
use tokio_stream::StreamExt;

pub enum Source {
    Readline(Readline),
    ReadDir(ReadDir),
    Nats(NatsConsumer),
}

impl Source {
    pub async fn new(config: &Config) -> Result<Self, SourceError> {
        let source = match &config.source {
            SourceTypes::ReadLine => Source::Readline(Readline()),
            SourceTypes::ReadDir { source_dir } => Source::ReadDir(ReadDir::new(source_dir)),
            SourceTypes::Nats {
                uri,
                subject,
                content_type,
            } => Source::Nats(NatsConsumer::new(uri, subject, content_type.clone())?),
        };
        Ok(source)
    }

    fn parse_blocks(buffer: impl AsRef<[u8]>) -> Result<SourceDataMessage, SourceError> {
        let blocks = Blocks::decode(buffer.as_ref())?;
        let decoded_blocks = blocks
            .ethereum_blocks
            .into_iter()
            .map(|b| Block::decode(b.as_slice()))
            .collect::<Result<Vec<Block>, _>>()
            .map_err(SourceError::DecodeError)?
            .into_iter()
            .map(EthereumFullBlock::try_from)
            .collect::<Result<Vec<EthereumFullBlock>, _>>()
            .map_err(SourceError::ParseDataError)?;

        Ok(SourceDataMessage::Protobuf(decoded_blocks))
    }

    pub async fn run_async(
        self,
        sender: AsyncSender<SourceDataMessage>,
    ) -> Result<(), SourceError> {
        match self {
            Source::Readline(source) => {
                let s = source.get_user_input_as_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(Self::parse_blocks(data)?).await?;
                }
            }
            Source::ReadDir(source) => {
                let s = source.get_json_in_dir_as_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(Self::parse_blocks(data)?).await?;
                }
            }
            Source::Nats(source) => {
                let s = source.get_subscription_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(Self::parse_blocks(data)?).await?;
                }
            }
        };

        Ok(())
    }
}
