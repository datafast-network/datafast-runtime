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
}

pub struct SubgraphTransportMessage {
    pub source: String,
    pub handler: String,
    pub data: SubgraphData,
}

pub struct Subgraph {
    pub sources: HashMap<String, SubgraphSource>,
    pub id: String,
    pub host: AscHost,
}

impl Subgraph {
    pub fn invoke(
        &mut self,
        source_id: &str,
        func: &str,
        data: SubgraphData,
    ) -> Result<(), SubgraphErr> {
        let source = self.sources.get(source_id).expect("Bad source id");
        let handler = source.handlers.get(func).expect("Bad handler name");

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
}
