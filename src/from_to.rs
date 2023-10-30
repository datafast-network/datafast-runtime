use crate::errors::SwrError;
use crate::host_exports::AscHost;
use crate::manifest_loader::DataSource;
use crate::subgraph::Handler;
use crate::subgraph::SubgraphSource;
use std::collections::HashMap;

impl TryFrom<(AscHost, DataSource)> for SubgraphSource {
    type Error = SwrError;

    fn try_from((host, source): (AscHost, DataSource)) -> Result<Self, Self::Error> {
        let mut handlers = HashMap::new();

        for (event_name, handler_name) in source.event_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(event_name.to_owned(), handler);
        }

        // FIXME: Saving the 2 following set of handlers this way can lead to NAMING-COLLISION
        for handler_name in source.block_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(handler_name.to_owned(), handler);
        }

        for (_, handler_name) in source.tx_handlers.iter() {
            let handler = Handler::new(&host.instance.exports, handler_name)?;
            handlers.insert(handler_name.to_owned(), handler);
        }

        let this = SubgraphSource {
            id: source.name.to_owned(),
            handlers,
            host,
        };

        Ok(this)
    }
}
