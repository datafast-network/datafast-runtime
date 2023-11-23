mod nats;
mod readdir;
mod readline;
mod trino;

use crate::components::source::nats::NatsConsumer;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use futures_util::pin_mut;
use kanal::AsyncSender;
use readdir::ReadDir;
use readline::Readline;
use tokio_stream::StreamExt;
use trino::TrinoClient;

pub enum Source {
    Readline(Readline),
    ReadDir(ReadDir),
    Nats(NatsConsumer),
    Trino(TrinoClient),
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
            SourceTypes::Trino {
                host,
                port,
                user,
                catalog,
                schema,
            } => Source::Trino(TrinoClient::new(host, port, user, catalog, schema)?),
        };
        Ok(source)
    }

    pub async fn run_async(
        self,
        sender: AsyncSender<SourceDataMessage>,
        sender2: AsyncSender<SerializedDataMessage>,
    ) -> Result<(), SourceError> {
        match self {
            Source::Readline(source) => {
                let s = source.get_user_input_as_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(data).await?;
                }
            }
            Source::ReadDir(source) => {
                let s = source.get_json_in_dir_as_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(data).await?;
                }
            }
            Source::Nats(source) => {
                let s = source.get_subscription_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender.send(data).await?;
                }
            }
            Source::Trino(source) => {
                let s = source.get_eth_block_stream().await;
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    sender2.send(data).await?;
                }
            }
        };

        Ok(())
    }
}
