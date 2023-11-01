use crate::common::Datasource;
use crate::errors::SwrError;
use crate::subgraph::DatasourceWasmInstance;
use crate::subgraph::Handler;
use crate::wasm_host::AscHost;
use std::collections::HashMap;

impl TryFrom<(AscHost, Datasource)> for DatasourceWasmInstance {
    type Error = SwrError;

    fn try_from((host, source): (AscHost, Datasource)) -> Result<Self, Self::Error> {
        let mut handlers = HashMap::new();
        let mapping = source.mapping;

        for event_handler in mapping.eventHandlers.unwrap_or_default().iter() {
            let handler = Handler::new(&host.instance.exports, &event_handler.handler)?;
            handlers.insert(event_handler.event.to_owned(), handler);
        }

        // FIXME: Saving the following set of handlers this way can lead to NAMING-COLLISION
        for block_handler in mapping.blockHandlers.unwrap_or_default().iter() {
            let handler = Handler::new(&host.instance.exports, &block_handler.handler)?;
            handlers.insert(block_handler.handler.to_owned(), handler);
        }

        Ok(DatasourceWasmInstance {
            id: source.name.to_owned(),
            handlers,
            host,
        })
    }
}
