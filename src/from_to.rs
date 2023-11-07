use crate::common::Datasource;
use crate::errors::SwrError;
use crate::subgraph::DatasourceWasmInstance;
use crate::subgraph::EthereumHandlers;
use crate::subgraph::Handler;
use crate::wasm_host::AscHost;
use std::collections::HashMap;

impl TryFrom<(AscHost, Datasource)> for DatasourceWasmInstance {
    type Error = SwrError;

    fn try_from((host, source): (AscHost, Datasource)) -> Result<Self, Self::Error> {
        let mut eth_event_handlers = HashMap::new();
        let mut eth_block_handlers = HashMap::new();

        let mapping = source.mapping;

        for event_handler in mapping.eventHandlers.unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-event handler, must fix later
            let handler = Handler::new(&host.instance.exports, &event_handler.handler)?;
            eth_event_handlers.insert(event_handler.event.to_owned(), handler);
        }

        for block_handler in mapping.blockHandlers.unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-block handler, must fix later
            let handler = Handler::new(&host.instance.exports, &block_handler.handler)?;
            eth_block_handlers.insert(block_handler.handler.to_owned(), handler);
        }

        Ok(DatasourceWasmInstance {
            id: source.name.to_owned(),
            ethereum_handlers: EthereumHandlers {
                block: eth_block_handlers,
                events: eth_event_handlers,
            },
            host,
        })
    }
}
