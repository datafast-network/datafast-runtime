/// This Source Mode is only used for testing / debugging
use crate::messages::SourceDataMessage;
use async_stream::stream;
use std::io;
use std::io::BufRead;
use tokio_stream::Stream;

pub struct Readline();

impl Readline {
    pub fn get_user_input_as_stream(self) -> impl Stream<Item = SourceDataMessage> {
        stream! {
            loop {
                let mut input = String::new();
                ::log::info!("Paste block data here...");
                let lines = io::stdin().lock().lines();

                for line in lines {
                    let last_input = line.unwrap();

                    if last_input.is_empty() {
                        break;
                    }

                    if !input.is_empty() {
                        input.push('\n');
                    }

                    input.push_str(&last_input);
                }

                match serde_json::from_str(&input) {
                    Ok(value) => yield SourceDataMessage::Json(value),
                    Err(_) => {
                        ::log::error!("Not json!");
                        continue;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_util::pin_mut;
    use tokio_stream::StreamExt;

    // NOTE: Interactive test only, not for CI
    #[tokio::test]
    async fn test_readline() {
        env_logger::try_init().unwrap_or_default();

        let rl = Readline();
        let stream = rl.get_user_input_as_stream();
        pin_mut!(stream);
        ::log::info!("Setup stream done");

        let t1 = async move {
            while let Some(data) = stream.next().await {
                ::log::info!("Received: {:?}", data);
            }
        };

        let _timeout = tokio::time::timeout(std::time::Duration::from_secs(3), t1);
        // timeout.await.unwrap();
    }
}
