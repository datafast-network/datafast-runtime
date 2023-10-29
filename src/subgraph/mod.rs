use crate::asc::base::asc_new;
use crate::asc::errors::AscError;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::host_exports::AscHost;
use kanal::Receiver;
use std::collections::HashMap;
use thiserror::Error;
use wasmer::Exports;
use wasmer::Function;
use wasmer::RuntimeError;
use wasmer::Value;
use web3::types::Log;

pub enum SubgraphData {
    Block(EthereumBlockData),
    Transaction(EthereumTransactionData),
    Event(EthereumEventData),
    Log(Log),
}

#[derive(Debug, Error)]
pub enum SubgraphErr {
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    AscError(#[from] AscError),
}

pub struct Handler {
    name: String,
    inner: Function,
}

impl Handler {
    pub fn new(instance_exports: &Exports, func_name: &str) -> Self {
        Self {
            name: func_name.to_string(),
            inner: instance_exports
                .get_function(&func_name)
                .expect("No function with such name exists")
                .to_owned(),
        }
    }
}

pub struct SubgraphSource {
    pub id: String,
    pub handlers: HashMap<String, Handler>,
    pub host: AscHost,
}

impl SubgraphSource {
    pub fn invoke(&mut self, func: &str, data: SubgraphData) -> Result<(), SubgraphErr> {
        let handler = self.handlers.get(func).expect("Bad handler name");

        match data {
            SubgraphData::Block(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner).unwrap();
                let ptr = asc_data.wasm_ptr() as i32;
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Transaction(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner).unwrap();
                let ptr = asc_data.wasm_ptr() as i32;
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Log(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner).unwrap();
                let ptr = asc_data.wasm_ptr() as i32;
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Event(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner).unwrap();
                let ptr = asc_data.wasm_ptr() as i32;
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
        }
    }
}

pub struct SubgraphTransportMessage {
    pub source: String,
    pub handler: String,
    pub data: SubgraphData,
}

pub struct Subgraph {
    pub sources: HashMap<String, SubgraphSource>,
    pub id: String,
}

impl Subgraph {
    pub fn invoke(
        &mut self,
        source_id: &str,
        func: &str,
        data: SubgraphData,
    ) -> Result<(), SubgraphErr> {
        let source = self.sources.get_mut(source_id).expect("Bad source id");
        source.invoke(func, data)
    }

    pub fn run_with_receiver(
        mut self,
        recv: Receiver<SubgraphTransportMessage>,
    ) -> Result<(), SubgraphErr> {
        while let Ok(msg) = recv.recv() {
            self.invoke(&msg.source, &msg.handler, msg.data)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    /* test flow
    - create multi source
    - bind to a single subgraph
    - invoke all handlers of sources
    */

    use std::collections::HashMap;

    use crate::host_exports::test::mock_host_instance;
    use crate::host_exports::test::version_to_test_resource;

    use super::Handler;
    use super::Subgraph;
    use super::SubgraphSource;

    #[::rstest::rstest]
    #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_subgraph(#[case] version: &str) {
        let mut subgraph = Subgraph {
            id: "TestSubgraph".to_string(),
            sources: HashMap::new(),
        };

        let subgraph_sources = vec![
            ("datasource1", "wasm_file_path1"),
            ("datasource2", "wasm_file_path2"),
        ];

        for (source_name, wasm_file_name) in subgraph_sources {
            std::env::set_var("TEST_WASM_FILE_NAME", wasm_file_name);
            let (version, wasm_path) = version_to_test_resource(version);

            let id = source_name.to_string();
            let host = mock_host_instance(version, &wasm_path);
            let handlers: HashMap<String, Handler> = [
                Handler::new(&host.instance.exports, "testS1Handler1"),
                Handler::new(&host.instance.exports, "testS2Handler2"),
                Handler::new(&host.instance.exports, "testS3Handler3"),
            ]
            .into_iter()
            .map(|h| (h.name.to_owned(), h))
            .collect();

            subgraph.sources.insert(
                source_name.to_string(),
                SubgraphSource { id, host, handlers },
            );
        }

        log::info!("Finished setup");
    }
}