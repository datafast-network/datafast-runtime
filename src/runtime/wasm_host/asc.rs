use crate::database::DatabaseAgent;
use crate::errors::AscError;
use crate::rpc_client::RpcAgent;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::IndexForAscTypeId;
use crate::runtime::wasm_host::Env;
use semver::Version;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::Mutex;
use wasmer::AsStoreMut;
use wasmer::AsStoreRef;
use wasmer::FromToNativeWasmType;
use wasmer::FunctionEnvMut;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::Store;
use wasmer::TypedFunction;
use wasmer::Value;

impl AscHeap for FunctionEnvMut<'_, Env> {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, AscError> {
        let (env, mut store) = self.data_and_store_mut();
        let size = i32::try_from(bytes.len()).unwrap();

        let mut arena_start_ptr = env
            .arena_start_ptr
            .lock()
            .expect("lock arena-start-ptr failed");

        let arena_size = size;

        // Unwrap: This may panic if more memory needs to be requested from the OS and that
        // fails. This error is not deterministic since it depends on the operating conditions
        // of the node.
        if let Some(memory_allocate) = env.memory_allocate.as_ref() {
            let new_arena_ptr = memory_allocate.call(&mut store, arena_size).unwrap();
            *arena_start_ptr = new_arena_ptr;
        }

        match &env.api_version {
            version if *version <= Version::new(0, 0, 4) => {}
            _ => {
                // This arithmetic is done because when you call AssemblyScripts's `__alloc`
                // function, it isn't typed and it just returns `mmInfo` on it's header,
                // differently from allocating on regular types (`__new` for example).
                // `mmInfo` has size of 4, and everything allocated on AssemblyScript memory
                // should have alignment of 16, this means we need to do a 12 offset on these
                // big chunks of untyped allocation.
                *arena_start_ptr += 12;
            }
        };

        let memory = env.memory.as_ref().unwrap();
        let view = memory.view(&store);
        // NOTE: write to page's footer
        let ptr = *arena_start_ptr as usize;
        view.write(ptr as u64, bytes)?;
        // Unwrap: We have just allocated enough space for `bytes`.
        *arena_start_ptr += size;

        Ok(ptr as u32)
    }

    fn read<'a>(
        &self,
        offset: u32,
        buffer: &'a mut [MaybeUninit<u8>],
    ) -> Result<&'a mut [u8], AscError> {
        let env = self.data();

        let memory = &env
            .memory
            .clone()
            .expect("(FunctionEnvMut::read) Memory must be initilized beforehand");

        let store_ref = self.as_store_ref();
        let view = memory.view(&store_ref);

        let result = view.read_uninit(offset as u64, buffer)?;
        Ok(result)
    }

    fn read_u32(&self, offset: u32) -> Result<u32, AscError> {
        let mut bytes = [0; 4];
        let env = self.data();
        assert!(env.memory.is_some(), "No memory???");
        let memory = &env
            .memory
            .clone()
            .expect("(FunctionEnvMut::read_u32) Memory must be initialized beforehand");

        let store_ref = self.as_store_ref();
        let view = memory.view(&store_ref);

        view.read(offset as u64, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn api_version(&self) -> Version {
        self.data().api_version.clone()
    }

    fn asc_type_id(&mut self, type_id_index: IndexForAscTypeId) -> Result<u32, AscError> {
        let (env, mut store_ref) = self.data_and_store_mut();
        env.id_of_type
            .as_ref()
            .unwrap() // Unwrap ok because it's only called on correct apiVersion, look for AscPtr::generate_header
            .call(&mut store_ref, type_id_index as u32)
            .map_err(|err| {
                AscError::Plain(format!(
                    "Failed to get Asc type id for index: {:?}. Trap: {}",
                    type_id_index, err
                ))
            })
    }
}

/// NOTE: This is not correct!
/// We basically just duplicate the impl AscHeap for FunctionEnvMut code above, which is kinda wrong
/// We should find a way to unify them
pub struct AscHost {
    pub store: Store,
    pub instance: Instance,
    pub memory: Memory,
    pub api_version: Version,
    pub id_of_type: Option<TypedFunction<u32, u32>>,
    pub memory_allocate: Option<TypedFunction<i32, i32>>,
    pub arena_start_ptr: Arc<Mutex<i32>>,
    pub db_agent: DatabaseAgent,
    pub rpc_agent: RpcAgent,
}

impl AscHeap for AscHost {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, AscError> {
        let mut arena_start_ptr = self
            .arena_start_ptr
            .lock()
            .expect("lock arena-start-ptr failed");

        let size = i32::try_from(bytes.len()).unwrap();
        let arena_size = size;

        if let Some(memory_allocate) = self.memory_allocate.clone() {
            let new_arena_ptr = memory_allocate.call(&mut self.store, arena_size).unwrap();
            *arena_start_ptr = new_arena_ptr;
        }

        match &self.api_version {
            version if *version <= Version::new(0, 0, 4) => {}
            _ => {
                *arena_start_ptr += 12;
            }
        };

        let view = self.memory.view(&self.store);
        let ptr = *arena_start_ptr as usize;
        view.write(ptr as u64, bytes)?;
        *arena_start_ptr += size;

        Ok(ptr as u32)
    }

    fn read<'a>(
        &self,
        offset: u32,
        buffer: &'a mut [MaybeUninit<u8>],
    ) -> Result<&'a mut [u8], AscError> {
        let store_ref = self.store.as_store_ref();
        let view = self.memory.view(&store_ref);

        let result = view.read_uninit(offset as u64, buffer)?;
        Ok(result)
    }

    fn read_u32(&self, offset: u32) -> Result<u32, AscError> {
        let mut bytes = [0; 4];
        let store_ref = self.store.as_store_ref();
        let view = self.memory.view(&store_ref);

        view.read(offset as u64, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn api_version(&self) -> Version {
        self.api_version.clone()
    }

    fn asc_type_id(&mut self, type_id_index: IndexForAscTypeId) -> Result<u32, AscError> {
        if self.id_of_type.is_none() {
            log::warn!("id_of_type is not available. skipping");
            return Ok(0);
        }

        self.id_of_type
            .as_ref()
            .unwrap()
            .call(&mut self.store.as_store_mut(), type_id_index as u32)
            .map_err(|err| {
                AscError::Plain(format!(
                    "Failed to get Asc type id for index: {:?}. Trap: {}",
                    type_id_index, err
                ))
            })
    }
}

unsafe impl<T> FromToNativeWasmType for AscPtr<T> {
    type Native = u32;

    #[inline]
    fn from_native(native: Self::Native) -> Self {
        AscPtr::<T>::new(native)
    }
    #[inline]
    fn to_native(self) -> Self::Native {
        self.wasm_ptr()
    }
}

impl<T> From<Value> for AscPtr<T> {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(n) => AscPtr::<T>::new(n as u32),
            _ => panic!("Cannot convert {:?} to AscPtr", value),
        }
    }
}
