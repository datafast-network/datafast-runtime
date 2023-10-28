use kanal::Receiver;
use std::collections::HashMap;
use thiserror::Error;
use wasmer::AsStoreMut;
use wasmer::AsStoreRef;
use wasmer::Exports;
use wasmer::FromToNativeWasmType;
use wasmer::Instance;
use wasmer::RuntimeError;
use wasmer::Store;
use wasmer::TypedFunction;

use crate::asc::errors::AscError;

#[derive(Debug, Error)]
pub enum SubgraphErr {
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    AscError(#[from] AscError),
}

pub trait ChainData: FromToNativeWasmType {}

pub struct Handler<T: ChainData> {
    name: String,
    inner: TypedFunction<T, ()>,
}

impl<T: ChainData> Handler<T> {
    pub fn new<S: AsStoreRef>(store: S, instance_exports: &Exports, func_name: &str) -> Self {
        Self {
            name: func_name.to_string(),
            inner: instance_exports
                .get_typed_function(&store, func_name)
                .expect("No function with such name exists"),
        }
    }
}

pub struct SubgraphSource<T: ChainData> {
    pub id: String,
    pub wasm_instance: Instance,
    pub handlers: HashMap<String, Handler<T>>,
    pub store: Store,
}

pub struct SubgraphTransportMessage<T> {
    pub source: String,
    pub handler: String,
    pub data: T,
}

pub struct Subgraph<T: ChainData> {
    pub sources: HashMap<String, SubgraphSource<T>>,
    pub id: String,
}

impl<T: ChainData> Subgraph<T> {
    pub fn invoke(&mut self, source_id: &str, func: &str, args: T) -> Result<(), SubgraphErr> {
        let source = self.sources.get_mut(source_id).expect("Bad source id");
        let mut store = source.store.as_store_mut();
        let handler = source.handlers.get(func).expect("Bad handler name");
        handler.inner.call(&mut store, args).map(Ok)?
    }

    pub fn run_with_receiver(
        mut self,
        recv: Receiver<SubgraphTransportMessage<T>>,
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
}
