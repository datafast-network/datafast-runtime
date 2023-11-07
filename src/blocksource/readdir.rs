use crate::messages::SourceDataMessage;
use async_stream::stream;
use std::fs::File;
use tokio_stream::Stream;

pub struct ReadDir {
    dir: String,
}

impl ReadDir {
    pub fn new(dir: &str) -> Self {
        ReadDir {
            dir: dir.to_owned(),
        }
    }

    pub fn get_json_in_dir_as_stream(self) -> impl Stream<Item = SourceDataMessage> {
        let paths = std::fs::read_dir(self.dir.clone())
            .unwrap()
            .flatten()
            .collect::<Vec<_>>();

        let mut json_files: Vec<String> = vec![];

        for path in paths {
            let ok_path = path.path();
            let extension = ok_path.extension().unwrap_or_default();
            let metadata = path.metadata();

            if metadata.is_err() {
                continue;
            }

            let metadata = metadata.unwrap();

            if !metadata.is_file() {
                continue;
            }

            if extension == "json" {
                json_files.push(path.path().to_str().unwrap().to_owned());
            }
        }

        stream! {
            for file in json_files {
                let file = File::open(file).unwrap();

                match serde_json::from_reader(&file) {
                    Ok(value) => yield SourceDataMessage::JSON(value),
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
    use tokio_stream::StreamExt;
    use web3::futures::pin_mut;

    #[tokio::test]
    async fn test_readdir() {
        ::env_logger::try_init().unwrap_or_default();

        let rd = ReadDir {
            dir: "/Users/vutran/Downloads".to_owned(),
        };
        let stream = rd.get_json_in_dir_as_stream();
        pin_mut!(stream);

        while let Some(data) = stream.next().await {
            ::log::info!("Received data: {:?}", data);
        }
    }
}
