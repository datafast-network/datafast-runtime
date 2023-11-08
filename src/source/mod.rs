mod readdir;
mod readline;

use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceErr;
use crate::messages::SourceDataMessage;
use async_stream::stream;
use futures_util::pin_mut;
use kanal::AsyncSender;
use readdir::ReadDir;
use readline::Readline;
use tokio::time;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

pub enum Source {
    Readline(Readline),
    ReadDir(ReadDir),
    Nats,
}

impl Source {
    pub fn new(config: &Config) -> Result<Self, SourceErr> {
        let source = match &config.source {
            SourceTypes::ReadLine => Source::Readline(Readline()),
            SourceTypes::ReadDir { source_dir } => Source::ReadDir(ReadDir::new(source_dir)),
            _ => unimplemented!(),
        };
        Ok(source)
    }
}

pub async fn block_stream(source: Source) -> impl Stream<Item = SourceDataMessage> {
    stream! {
        match source {
            Source::Readline(source) => {
                let s = source.get_user_input_as_stream();
                pin_mut!(s);
                while let Some(data) = s.next().await {
                    yield data;
                };
            }
            Source::ReadDir(source) => {
                let s = source.get_json_in_dir_as_stream();
                pin_mut!(s);
                let start = time::Instant::now();
                while let Some(data) = s.next().await {
                    yield data;
                    log::info!("elapsed read data: {:?}", start.elapsed());
                };
            }
            _ => unimplemented!(),
        }
    }
}

pub async fn stream_consume<T: Stream<Item = SourceDataMessage>>(
    stream: T,
    source_sender: AsyncSender<SourceDataMessage>,
) -> Result<(), SourceErr> {
    pin_mut!(stream);

    while let Some(data) = stream.next().await {
        source_sender.send(data).await?;
    }

    Ok(())
}
