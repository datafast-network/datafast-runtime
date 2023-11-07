use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscType;
use crate::asc::base::ToAscObj;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::common::HandlerTypes;
use crate::errors::SubgraphError;
use crate::messages::EthereumFilteredEvent;
use crate::messages::FilteredDataMessage;
use crate::wasm_host::AscHost;
use kanal::AsyncReceiver;
use std::collections::HashMap;
use wasmer::Exports;
use wasmer::Function;
use wasmer::Value;

pub struct Handler {
    pub name: String,
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

pub struct EthereumHandlers {
    pub block: HashMap<String, Handler>,
    pub events: HashMap<String, Handler>,
}

pub struct DatasourceWasmInstance {
    pub id: String,
    // NOTE: Add more chain-based handler here....
    pub ethereum_handlers: EthereumHandlers,
    pub host: AscHost,
}

impl DatasourceWasmInstance {
    pub fn invoke<T: AscType + AscIndexId>(
        &mut self,
        handler_type: HandlerTypes,
        handler_name: &str,
        mut data: impl ToAscObj<T>,
    ) -> Result<(), SubgraphError> {
        let handler = match handler_type {
            HandlerTypes::EthereumBlock => self.ethereum_handlers.block.get(handler_name),
            HandlerTypes::EthereumEvent => self.ethereum_handlers.events.get(handler_name),
            _ => {
                unimplemented!()
            }
        }
        .ok_or(SubgraphError::InvalidHandlerName(handler_name.to_owned()))?;

        let asc_data = asc_new(&mut self.host, &mut data)?;
        handler.inner.call(
            &mut self.host.store,
            &[Value::I32(asc_data.wasm_ptr() as i32)],
        )?;
        Ok(())
    }
}

pub struct Subgraph {
    // NOTE: using IPFS might lead to subgraph-id using a hex/hash
    pub id: String,
    pub name: String,
    sources: HashMap<String, DatasourceWasmInstance>,
}

impl Subgraph {
    pub fn new_empty(name: &str, id: String) -> Self {
        Self {
            sources: HashMap::new(),
            name: name.to_owned(),
            id,
        }
    }

    pub fn add_source(&mut self, source: DatasourceWasmInstance) -> bool {
        self.sources.insert(source.id.clone(), source).is_some()
    }

    fn handle_ethereum_filtered_data(
        &mut self,
        events: Vec<EthereumFilteredEvent>,
        block: EthereumBlockData,
    ) -> Result<(), SubgraphError> {
        let mut block_handlers = HashMap::new();

        for (source_name, source_instance) in self.sources.iter() {
            let source_block_handlers = source_instance
                .ethereum_handlers
                .block
                .keys()
                .cloned()
                .collect::<Vec<String>>();
            block_handlers.insert(source_name.to_owned(), source_block_handlers);
        }

        for (source_name, handlers) in block_handlers {
            // FIXME: this is not correct, block-handler may have filter itself,
            // thus not all datasource would handle the same block
            let source_instance = self.sources.get_mut(&source_name).unwrap();
            for handler in handlers {
                source_instance.invoke(HandlerTypes::EthereumBlock, &handler, block.clone())?;
            }
        }

        for event in events {
            let source_instance = self
                .sources
                .get_mut(&event.datasource)
                .ok_or(SubgraphError::InvalidSourceID(event.datasource.to_owned()))?;
            source_instance.invoke(HandlerTypes::EthereumEvent, &event.handler, event.event)?;
        }

        Ok(())
    }

    fn handle_filtered_data(&mut self, data: FilteredDataMessage) -> Result<(), SubgraphError> {
        match data {
            FilteredDataMessage::Ethereum { events, block } => {
                self.handle_ethereum_filtered_data(events, block)
            }
            _ => {
                unimplemented!()
            }
        }
    }

    pub async fn run_async(
        &mut self,
        recv: AsyncReceiver<FilteredDataMessage>,
    ) -> Result<(), SubgraphError> {
        while let Ok(msg) = recv.recv().await {
            self.handle_filtered_data(msg)?;
        }

        Ok(())
    }
}
//
// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::chain::ethereum::block::EthereumBlockData;
//     use crate::chain::ethereum::event::EthereumEventData;
//     use crate::chain::ethereum::transaction::EthereumTransactionData;
//     use crate::messages::SubgraphJob;
//     use crate::messages::SubgraphOperationMessage;
//     use crate::wasm_host::test::get_subgraph_testing_resource;
//     use crate::wasm_host::test::mock_wasm_host;
//     use ethabi::ethereum_types::H160;
//     use ethabi::ethereum_types::U256;
//     use std::collections::HashMap;
//     use std::str::FromStr;
//     use std::thread;
//
//     #[::rstest::rstest]
//     #[case("0.0.4")]
//     #[case("0.0.5")]
//     fn test_subgraph(#[case] version: &str) {
//         env_logger::try_init().unwrap_or_default();
//
//         let mut subgraph = Subgraph {
//             id: "TestSubgraph".to_string(),
//             name: "TestSubgraph".to_string(),
//             sources: HashMap::new(),
//         };
//
//         let subgraph_sources = vec!["TestDataSource1"];
//
//         for source_name in subgraph_sources {
//             let (version, wasm_path) = get_subgraph_testing_resource(version, "TestDataSource");
//
//             let id = source_name.to_string();
//             let host = mock_wasm_host(version.clone(), &wasm_path);
//             let mut handlers: HashMap<String, Handler> = [
//                 Handler::new(&host.instance.exports, "testHandlerBlock").unwrap(),
//                 Handler::new(&host.instance.exports, "testHandlerEvent").unwrap(),
//                 // Do not add these entry to subgraph.yaml, and everything can run just fine
//                 Handler::new(&host.instance.exports, "testHandlerTransaction").unwrap(),
//             ]
//             .into_iter()
//             .map(|h| (h.name.to_owned(), h))
//             .collect();
//
//             if version.patch == 5 {
//                 // NOTE: v0_0_4 does not support Log type
//                 handlers.insert(
//                     "testHandlerLog".to_string(),
//                     Handler::new(&host.instance.exports, "testHandlerLog").unwrap(),
//                 );
//             }
//
//             subgraph.sources.insert(
//                 source_name.to_string(),
//                 DatasourceWasmInstance { id, host, handlers },
//             );
//         }
//
//         log::info!("Finished setup");
//
//         let (sender, receiver) = kanal::bounded(1);
//
//         let t = thread::spawn(move || {
//             if let Err(e) = subgraph.run_with_receiver(receiver) {
//                 log::error!("Run subgraph with receiver failed: {:?}", e);
//             }
//         });
//
//         // Test sending block data
//         let block_data_msg = SubgraphJob {
//             source: "TestDataSource1".to_string(),
//             handler: "testHandlerBlock".to_string(),
//             data: SubgraphData::Block(EthereumBlockData::default()),
//         };
//         log::info!("------- Send block to blockHandler of Subgraph");
//         sender
//             .send(SubgraphOperationMessage::Job(block_data_msg))
//             .expect("Failed to send block_data_msg");
//
//         // Test sending event data
//         let event_data_msg = SubgraphJob {
//             source: "TestDataSource1".to_string(),
//             handler: "testHandlerEvent".to_string(),
//             data: SubgraphData::Event(EthereumEventData {
//                 block: EthereumBlockData {
//                     number: ethabi::ethereum_types::U64::from(1000),
//                     ..Default::default()
//                 },
//                 ..Default::default()
//             }),
//         };
//         log::info!("------- Send event to eventHandler of Subgraph");
//         sender
//             .send(SubgraphOperationMessage::Job(event_data_msg))
//             .expect("Failed to send event_data_msg");
//
//         // Test sending tx data
//         let transaction_data_msg = SubgraphJob {
//             source: "TestDataSource1".to_string(),
//             handler: "testHandlerTransaction".to_string(),
//             data: SubgraphData::Transaction(EthereumTransactionData {
//                 from: H160::from_str("0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326").unwrap(),
//                 to: Some(H160::from_str("0x388C818CA8B9251b393131C08a736A67ccB19297").unwrap()),
//                 value: U256::from(10000),
//                 ..Default::default()
//             }),
//         };
//         log::info!("------- Send transaction to transactionHandler of Subgraph");
//         sender
//             .send(SubgraphOperationMessage::Job(transaction_data_msg))
//             .expect("Failed to send transaction_data_msg");
//
//         // Shutting down subgraph
//         log::info!("------- Send request to close subgraph");
//         sender.send(SubgraphOperationMessage::Finish).unwrap();
//
//         t.join().unwrap();
//     }
// }
