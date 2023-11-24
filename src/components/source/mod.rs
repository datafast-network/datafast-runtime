mod nats;
mod readdir;
mod readline;
mod trino;

use crate::common::Chain;
use crate::components::source::nats::NatsConsumer;
use crate::components::ProgressCtrl;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use crate::messages::SourceDataMessage;
use futures_util::pin_mut;
use kanal::bounded_async;
use kanal::AsyncSender;
use readdir::ReadDir;
use readline::Readline;
use tokio_stream::StreamExt;
use trino::TrinoClient;
use trino::TrinoEthereumBlock;

enum Source {
    Readline(Readline),
    ReadDir(ReadDir),
    Nats(NatsConsumer),
    Trino(TrinoClient),
}

pub struct BlockSource {
    source: Source,
    chain: Chain,
}

impl BlockSource {
    pub async fn new(config: &Config, pctrl: ProgressCtrl) -> Result<Self, SourceError> {
        let start_block = pctrl.get_min_start_block();
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
                table,
                query_step,
            } => Source::Trino(TrinoClient::new(
                host,
                port,
                user,
                catalog,
                schema,
                table,
                start_block,
                query_step.to_owned(),
            )?),
        };
        Ok(Self {
            source,
            chain: config.chain.clone(),
        })
    }

    pub async fn run_async(
        self,
        sender: AsyncSender<SourceDataMessage>,
        sender2: AsyncSender<SerializedDataMessage>,
    ) -> Result<(), SourceError> {
        match self.source {
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
                let (trino_blocks_sender, trino_blocks_receiver) = bounded_async(1);

                let query_blocks = match self.chain {
                    Chain::Ethereum => {
                        source.get_block_stream::<TrinoEthereumBlock>(trino_blocks_sender)
                    }
                };

                let handle_received_blockss = async {
                    while let Ok(blocks) = trino_blocks_receiver.recv().await {
                        for block in blocks {
                            sender2.send(block).await.unwrap();
                        }
                    }
                };

                tokio::select! {
                    _ = query_blocks => (),
                    _ = handle_received_blockss => ()
                };
            }
        };

        Ok(())
    }
}
