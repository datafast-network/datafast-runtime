use crate::common::Datasource;
use crate::common::DatasourceBundle;
use crate::common::HandlerTypes;
use crate::components::ManifestAgent;
use crate::database::DatabaseAgent;
use crate::errors::SubgraphError;
use crate::rpc_client::RpcAgent;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::ToAscObj;
use crate::runtime::wasm_host::AscHost;
use std::collections::HashMap;
use wasmer::Exports;
use wasmer::Function;
use wasmer::Value;
use crate::store_filter::StoreFilter;

pub struct Handler {
    pub name: String,
    inner: Function,
}

impl Handler {
    pub fn new(instance_exports: &Exports, func_name: &str) -> Result<Self, SubgraphError> {
        let this = Self {
            name: func_name.to_string(),
            inner: instance_exports
                .get_function(func_name)
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
    pub name: String,
    // NOTE: Add more chain-based handler here....
    pub ethereum_handlers: EthereumHandlers,
    host: AscHost,
}

impl TryFrom<(&AscHost, &Datasource)> for EthereumHandlers {
    type Error = SubgraphError;
    fn try_from((host, ds): (&AscHost, &Datasource)) -> Result<Self, SubgraphError> {
        let mut eth_event_handlers = HashMap::new();
        let mut eth_block_handlers = HashMap::new();

        for event_handler in ds.mapping.eventHandlers.clone().unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-event handler, must fix later
            let handler = Handler::new(&host.instance.exports, &event_handler.handler)?;
            eth_event_handlers.insert(event_handler.handler.to_owned(), handler);
        }

        for block_handler in ds.mapping.blockHandlers.clone().unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-block handler, must fix later
            let handler = Handler::new(&host.instance.exports, &block_handler.handler)?;
            eth_block_handlers.insert(block_handler.handler.to_owned(), handler);
        }

        Ok(EthereumHandlers {
            block: eth_block_handlers,
            events: eth_event_handlers,
        })
    }
}

impl TryFrom<(DatasourceBundle, DatabaseAgent, RpcAgent, ManifestAgent, Option<StoreFilter>)>
    for DatasourceWasmInstance
{
    type Error = SubgraphError;
    fn try_from(
        value: (DatasourceBundle, DatabaseAgent, RpcAgent, ManifestAgent, Option<StoreFilter>),
    ) -> Result<Self, Self::Error> {
        let host = AscHost::try_from(value.clone())
            .map_err(|e| SubgraphError::CreateSourceFail(e.to_string()))?;
        let ethereum_handlers = EthereumHandlers::try_from((&host, &value.0.ds))?;
        let name = value.0.name();
        Ok(Self {
            host,
            name,
            ethereum_handlers,
        })
    }
}

impl DatasourceWasmInstance {
    const MAXIMUM_HEAP_SIZE: f32 = 0.5 * (i32::MAX as f32);

    pub fn invoke<T: AscType + AscIndexId>(
        &mut self,
        handler_type: HandlerTypes,
        handler_name: &str,
        data: impl ToAscObj<T>,
    ) -> Result<(), SubgraphError> {
        let handler = match handler_type {
            HandlerTypes::EthereumBlock => self.ethereum_handlers.block.get(handler_name),
            HandlerTypes::EthereumEvent => self.ethereum_handlers.events.get(handler_name),
        }
        .ok_or(SubgraphError::InvalidHandlerName(handler_name.to_owned()))?;

        let asc_data = asc_new(&mut self.host, &data)?;
        handler.inner.call(
            &mut self.host.store,
            &[Value::I32(asc_data.wasm_ptr() as i32)],
        )?;

        Ok(())
    }

    pub fn should_reset(&self) -> bool {
        (self.host.current_ptr() as f32) > Self::MAXIMUM_HEAP_SIZE
    }
}
