use semver::Version;
use wasmer::{Instance, Store};
use crate::global::GlobalMemory;

pub struct WasmContext {
    heap: GlobalMemory,
}

impl WasmContext {
    pub fn new(instance: &Instance, store: &Store, api_version: Version) -> Result<Self,  Box<dyn std::error::Error>> {
        let memory = instance.exports.get_memory("memory")?.clone();
        let memory_allocate = match api_version.clone() {
            version if version <= Version::new(0, 0, 4) => instance.exports.get_function("memory.allocate")?.typed(store)?,
            _ => instance.exports.get_function("allocate")?.typed(&store)?,
        };
        let id_of_type = match api_version.clone() {
            version if version <= Version::new(0, 0, 4) => instance.exports.get_function("id_of_type")?.typed::<u32, u32>(store)?.clone(),
            _ => instance.exports.get_function("id_of_type")?.typed::<u32, u32>(store)?.clone()
        };
        let heap = GlobalMemory {
            memory,
            memory_allocate,
            id_of_type,
            api_version,
            arena_free_size: 0,
            arena_start_ptr: 0,
        };
        Ok( Self {
            heap,
        })
    }

    pub fn get_heap(&self) -> &GlobalMemory {
        &self.heap
    }

    pub fn get_heap_mut(&mut self) -> &mut GlobalMemory {
        &mut self.heap
    }
}