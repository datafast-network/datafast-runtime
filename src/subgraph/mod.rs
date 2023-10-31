use crate::asc::base::asc_new;
use crate::errors::SubgraphError;
use crate::internal_messages::SubgraphData;
use crate::internal_messages::SubgraphOperationMessage;
use crate::wasm_host::AscHost;
use kanal::AsyncReceiver;
use kanal::Receiver;
use std::collections::HashMap;
use wasmer::Exports;
use wasmer::Function;
use wasmer::Value;

pub struct Handler {
    name: String,
    inner: Function,
}

impl Handler {
    pub fn new(instance_exports: &Exports, func_name: &str) -> Result<Self, SubgraphError> {
        let this = Self {
            name: func_name.to_string(),
            inner: instance_exports
                .get_function(&func_name)
                .map_err(|_| SubgraphError::InvalidHandlerName(func_name.to_owned()))?
                .to_owned(),
        };
        Ok(this)
    }
}

pub struct SubgraphSource {
    pub id: String,
    pub handlers: HashMap<String, Handler>,
    pub host: AscHost,
}

impl SubgraphSource {
    pub fn invoke(&mut self, func: &str, data: SubgraphData) -> Result<(), SubgraphError> {
        log::info!("Source={} is invoking function{func}", self.id);
        let handler = self
            .handlers
            .get(func)
            .ok_or_else(|| SubgraphError::Plain("Bad handler name".to_string()))?;

        match data {
            SubgraphData::Block(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner)?;
                let ptr = asc_data.wasm_ptr() as i32;
                log::info!("Calling block handler");
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Transaction(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner)?;
                let ptr = asc_data.wasm_ptr() as i32;
                log::info!("Calling tx handler");
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Log(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner)?;
                let ptr = asc_data.wasm_ptr() as i32;
                log::info!("Calling log handler");
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
            SubgraphData::Event(mut inner) => {
                let asc_data = asc_new(&mut self.host, &mut inner)?;
                let ptr = asc_data.wasm_ptr() as i32;
                log::info!("Calling event handler");
                handler
                    .inner
                    .call(&mut self.host.store, &[Value::I32(ptr)])?;
                Ok(())
            }
        }
    }
}

pub struct Subgraph<T: ToString> {
    pub sources: HashMap<String, SubgraphSource>,
    pub id: T,
    pub name: String,
}

// NOTE: using IPFS might lead to subgraph-id using a hex/hash
impl<T: ToString> Subgraph<T> {
    pub fn new_empty(name: &str, id: T) -> Self {
        Self {
            sources: HashMap::new(),
            name: name.to_owned(),
            id,
        }
    }

    pub fn add_source(&mut self, source: SubgraphSource) -> bool {
        self.sources.insert(source.id.clone(), source).is_some()
    }

    pub fn invoke(
        &mut self,
        source_id: &str,
        func: &str,
        data: SubgraphData,
    ) -> Result<(), SubgraphError> {
        log::info!("Invoking: source={source_id}, func={func}");
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or_else(|| SubgraphError::InvalidSourceID(source_id.to_owned()))?;
        source.invoke(func, data)
    }

    pub fn run_with_receiver(
        mut self,
        recv: Receiver<SubgraphOperationMessage>,
    ) -> Result<(), SubgraphError> {
        while let Ok(op) = recv.recv() {
            match op {
                SubgraphOperationMessage::Job(msg) => {
                    log::info!("Received msg: {:?}", msg);
                    self.invoke(&msg.source, &msg.handler, msg.data)?;
                }
                SubgraphOperationMessage::Finish => {
                    log::info!("Request to shutdown Subgraph");
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    pub async fn run_async(
        mut self,
        recv: AsyncReceiver<SubgraphOperationMessage>,
    ) -> Result<(), SubgraphError> {
        while let Ok(op) = recv.recv().await {
            match op {
                SubgraphOperationMessage::Job(msg) => {
                    log::info!("Received msg: {:?}", msg);
                    self.invoke(&msg.source, &msg.handler, msg.data)?;
                }
                SubgraphOperationMessage::Finish => {
                    log::info!("Request to shutdown Subgraph");
                    return Ok(());
                }
            }
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

    use super::*;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::chain::ethereum::event::EthereumEventData;
    use crate::chain::ethereum::transaction::EthereumTransactionData;
    use crate::internal_messages::SubgraphJob;
    use crate::internal_messages::SubgraphOperationMessage;
    use crate::wasm_host::test::mock_host_instance;
    use crate::wasm_host::test::version_to_test_resource;
    use ethabi::ethereum_types::H160;
    use ethabi::ethereum_types::U256;
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::thread;

    #[::rstest::rstest]
    #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_subgraph(#[case] version: &str) {
        env_logger::try_init().unwrap_or_default();

        let mut subgraph = Subgraph {
            id: "TestSubgraph".to_string(),
            name: "TestSubgraph".to_string(),
            sources: HashMap::new(),
        };

        let subgraph_sources = vec!["TestDataSource1"];

        for source_name in subgraph_sources {
            let (version, wasm_path) = version_to_test_resource(version, "datasource");

            let id = source_name.to_string();
            let host = mock_host_instance(version.clone(), &wasm_path);
            let mut handlers: HashMap<String, Handler> = [
                Handler::new(&host.instance.exports, "testHandlerBlock").unwrap(),
                Handler::new(&host.instance.exports, "testHandlerEvent").unwrap(),
                // Do not add these entry to subgraph.yaml, and everything can run just fine
                Handler::new(&host.instance.exports, "testHandlerTransaction").unwrap(),
            ]
            .into_iter()
            .map(|h| (h.name.to_owned(), h))
            .collect();

            if version.patch == 5 {
                // NOTE: v0_0_4 does not support Log type
                handlers.insert(
                    "testHandlerLog".to_string(),
                    Handler::new(&host.instance.exports, "testHandlerLog").unwrap(),
                );
            }

            subgraph.sources.insert(
                source_name.to_string(),
                SubgraphSource { id, host, handlers },
            );
        }

        log::info!("Finished setup");

        let (sender, receiver) = kanal::bounded(1);

        let t = thread::spawn(move || {
            if let Err(e) = subgraph.run_with_receiver(receiver) {
                log::error!("Run subgraph with receiver failed: {:?}", e);
            }
        });

        // Test sending block data
        let block_data_msg = SubgraphJob {
            source: "TestDataSource1".to_string(),
            handler: "testHandlerBlock".to_string(),
            data: crate::subgraph::SubgraphData::Block(EthereumBlockData::default()),
        };
        log::info!("------- Send block to blockHandler of Subgraph");
        sender
            .send(SubgraphOperationMessage::Job(block_data_msg))
            .expect("Failed to send block_data_msg");

        // Test sending event data
        let event_data_msg = SubgraphJob {
            source: "TestDataSource1".to_string(),
            handler: "testHandlerEvent".to_string(),
            data: crate::subgraph::SubgraphData::Event(EthereumEventData {
                block: EthereumBlockData {
                    number: ethabi::ethereum_types::U64::from(1000),
                    ..Default::default()
                },
                ..Default::default()
            }),
        };
        log::info!("------- Send event to eventHandler of Subgraph");
        sender
            .send(SubgraphOperationMessage::Job(event_data_msg))
            .expect("Failed to send event_data_msg");

        // Test sending tx data
        let transaction_data_msg = SubgraphJob {
            source: "TestDataSource1".to_string(),
            handler: "testHandlerTransaction".to_string(),
            data: crate::subgraph::SubgraphData::Transaction(EthereumTransactionData {
                from: H160::from_str("0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326").unwrap(),
                to: Some(H160::from_str("0x388C818CA8B9251b393131C08a736A67ccB19297").unwrap()),
                value: U256::from(10000),
                ..Default::default()
            }),
        };
        log::info!("------- Send transaction to transactionHandler of Subgraph");
        sender
            .send(SubgraphOperationMessage::Job(transaction_data_msg))
            .expect("Failed to send transaction_data_msg");

        // Shutting down subgraph
        log::info!("------- Send request to close subgraph");
        sender.send(SubgraphOperationMessage::Finish).unwrap();

        t.join().unwrap();
    }
}
