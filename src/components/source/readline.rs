/// This Source Mode is only used for testing / debugging
use async_stream::stream;
use std::io::BufRead;
use std::io::{self};
use tokio_stream::Stream;
pub struct Readline();

impl Readline {
    pub fn get_user_input_as_stream(self) -> impl Stream<Item = Vec<u8>> {
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

                yield input.into_bytes()
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
