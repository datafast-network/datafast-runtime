use crate::common::Datasource;
use crate::common::HandlerTypes;
use crate::errors::SubgraphError;
use crate::log_info;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::ToAscObj;
use crate::runtime::wasm_host::AscHost;
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
        log_info!(DatasourceWasmInstance, "Handler invoked";
            handler => handler.name.clone(),
            handler_type => format!("{:?}", handler_type));
        Ok(())
    }
}

impl TryFrom<(AscHost, Datasource)> for DatasourceWasmInstance {
    type Error = SubgraphError;

    fn try_from((host, source): (AscHost, Datasource)) -> Result<Self, Self::Error> {
        let mut eth_event_handlers = HashMap::new();
        let mut eth_block_handlers = HashMap::new();

        let mapping = source.mapping;

        for event_handler in mapping.eventHandlers.unwrap_or_default().iter() {
            // FIXME: assuming handlers are ethereum-event handler, must fix later
            let handler = Handler::new(&host.instance.exports, &event_handler.handler)?;
            eth_event_handlers.insert(event_handler.handler.to_owned(), handler);
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
