use wasmer::Memory;
use wasmer::RuntimeError;
use wasmer::Store;
use wasmer::Type;
use wasmer::TypedFunction;
use wasmer::Value;

pub const ABORT_TYPE: ([Type; 4], [Type; 0]) = ([Type::I32, Type::I32, Type::I32, Type::I32], []);

pub fn abort(messages: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    //convert message to string
    println!("ABORT: {:?}", messages);
    let message = messages
        .iter()
        .fold(String::new(), |acc, val| acc + &val.to_string());
    println!("ABORT: {}", message);
    Ok(vec![])
}

pub struct GlobalMemory {
    pub memory: Memory,
    pub memory_allocate: TypedFunction<i32, i32>,
    pub id_of_type: TypedFunction<u32, u32>,
    pub arena_free_size: i32,
    pub arena_start_ptr: i32,
    // pub api_version: Version,
}

impl GlobalMemory {
    fn read_u32(&self, offset: u32) -> Result<u32, String> {
        let mut bytes = [0; 4];
        //
        // self.memory.read(offset as usize, &mut bytes).map_err(|_| {
        //     format!("Failed to read u32 from memory at offset {}", offset)
        // })?;
        Ok(u32::from_le_bytes(bytes))
    }
}
