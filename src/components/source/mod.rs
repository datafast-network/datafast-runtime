mod nats;
mod readdir;
mod readline;

use crate::components::source::nats::NatsConsumer;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceErr;
use crate::messages::SourceDataMessage;
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
    pub async fn new(config: &Config) -> Result<Self, SourceErr> {
        let source = match &config.source {
            SourceTypes::ReadLine => Source::Readline(Readline()),
            SourceTypes::ReadDir { source_dir } => Source::ReadDir(ReadDir::new(source_dir)),
            _ => unimplemented!(),
        };
        Ok(source)
    }

    pub async fn run_async(self, sender: AsyncSender<SourceDataMessage>) -> Result<(), SourceErr> {
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
                source.run_async(sender).await?;
            }
        };

        Ok(())
    }
}
