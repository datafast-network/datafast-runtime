use std::collections::HashMap;
use std::marker::PhantomData;
use wasmer::Instance;
use wasmer::TypedFunction;
use wasmer::Value;

pub enum HandlerDataType {
    Block,
    Transaction,
    Event,
    Log,
}

pub struct Handler<T> {
    inner: TypedFunction<[Value; 1], [Value; 0]>,
    ty: PhantomData<T>,
}

pub struct SubgraphSource {
    pub id: String,
    pub wasm_instance: Instance,
    pub handlers: HashMap<String, Handler<HandlerDataType>>,
}

pub struct Subgraph(HashMap<String, SubgraphSource>);
