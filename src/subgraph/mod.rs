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

#[derive(Debug)]
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

#[derive(Debug)]
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
            log::info!("Received msg: {:?}", msg);
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

    use super::Handler;
    use super::Subgraph;
    use super::SubgraphSource;
    use super::SubgraphTransportMessage;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::host_exports::test::mock_host_instance;
    use crate::host_exports::test::version_to_test_resource;
    use std::collections::HashMap;
    use std::thread;

    #[::rstest::rstest]
    #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_subgraph(#[case] version: &str) {
        env_logger::try_init().unwrap_or_default();

        let mut subgraph = Subgraph {
            id: "TestSubgraph".to_string(),
            sources: HashMap::new(),
        };

        let subgraph_sources = vec![("datasource-test", "datasource")];

        for (source_name, wasm_file_name) in subgraph_sources {
            std::env::set_var("TEST_WASM_FILE_NAME", wasm_file_name);
            let (version, wasm_path) = version_to_test_resource(version);

            let id = source_name.to_string();
            let host = mock_host_instance(version, &wasm_path);
            let handlers: HashMap<String, Handler> = [
                Handler::new(&host.instance.exports, "testHandlerBlock"),
                // Handler::new(&host.instance.exports, "testHandlerTransaction"),
                // Handler::new(&host.instance.exports, "testHandlerLog"),
                Handler::new(&host.instance.exports, "testHandlerEvent"),
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

        let (sender, receiver) = kanal::bounded(1);

        thread::spawn(move || {
            subgraph.run_with_receiver(receiver).unwrap();
        });

        let msg1 = SubgraphTransportMessage {
            source: "TestDatasource1".to_string(),
            handler: "testHandleBlock".to_string(),
            data: crate::subgraph::SubgraphData::Block(EthereumBlockData::default()),
        };

        sender.send(msg1).unwrap();
    }
}
