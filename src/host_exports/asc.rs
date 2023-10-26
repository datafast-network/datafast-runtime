use crate::asc::base::AscHeap;
use crate::asc::base::AscPtr;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::AscError;
use crate::host_exports::Env;
use semver::Version;
use std::mem::MaybeUninit;
use wasmer::AsStoreRef;
use wasmer::FromToNativeWasmType;
use wasmer::FunctionEnvMut;
use wasmer::Value;

const MIN_ARENA_SIZE: i32 = 10_000;

impl AscHeap for FunctionEnvMut<'_, Env> {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, AscError> {
        let require_length = bytes.len() as u64;
        let (env, mut store) = self.data_and_store_mut();
        let size = i32::try_from(bytes.len()).unwrap();

        if size > env.arena_free_size {
            // Allocate a new arena. Any free space left in the previous arena is left unused. This
            // causes at most half of memory to be wasted, which is acceptable.
            let arena_size = size.max(MIN_ARENA_SIZE);

            // Unwrap: This may panic if more memory needs to be requested from the OS and that
            // fails. This error is not deterministic since it depends on the operating conditions
            // of the node.
            if let Some(memory_allocate) = env.memory_allocate.as_ref() {
                let new_arena_ptr = memory_allocate.call(&mut store, arena_size).unwrap();
                env.arena_start_ptr = new_arena_ptr;
            }

            env.arena_free_size = arena_size;

            match &env.api_version {
                version if *version <= Version::new(0, 0, 4) => {}
                _ => {
                    // This arithmetic is done because when you call AssemblyScripts's `__alloc`
                    // function, it isn't typed and it just returns `mmInfo` on it's header,
                    // differently from allocating on regular types (`__new` for example).
                    // `mmInfo` has size of 4, and everything allocated on AssemblyScript memory
                    // should have alignment of 16, this means we need to do a 12 offset on these
                    // big chunks of untyped allocation.
                    env.arena_start_ptr += 12;
                    env.arena_free_size -= 12;
                }
            };
        };

        let memory = env.memory.as_ref().unwrap();
        let view = memory.view(&store);
        let available_length = view.data_size();

        // For now, not allow increase memory size
        if available_length < require_length {
            return Err(AscError::SizeNotFit);
        }

        // NOTE: write to page's footer
        let ptr = env.arena_start_ptr as usize;

        view.write(ptr as u64, bytes).expect("Failed");

        // Unwrap: We have just allocated enough space for `bytes`.
        env.arena_start_ptr += size;
        env.arena_free_size -= size;

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
            .expect("Memory must be initilized beforehand");

        let store_ref = self.as_store_ref();
        let view = memory.view(&store_ref);

        view.read_uninit(offset as u64, buffer)
            .map_err(|_| AscError::Plain(format!("Heap access out of bounds. Offset: {}", offset)))
    }

    fn read_u32(&self, offset: u32) -> Result<u32, AscError> {
        let mut bytes = [0; 4];
        let env = self.data();

        let memory = &env
            .memory
            .clone()
            .expect("Memory must be initilized beforehand");

        let store_ref = self.as_store_ref();
        let view = memory.view(&store_ref);

        view.read(offset as u64, &mut bytes).map_err(|_| {
            AscError::Plain(format!(
                "Heap access out of bounds. Offset: {} Size: {}",
                offset, 4
            ))
        })?;
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

#[cfg(test)]
pub mod test {
    use crate::asc::base::AscHeap;
    use crate::asc::base::IndexForAscTypeId;
    use crate::asc::errors::AscError;
    use semver::Version;
    use std::mem::MaybeUninit;
    use wasmer::AsStoreMut;
    use wasmer::AsStoreRef;
    use wasmer::Instance;
    use wasmer::Memory;
    use wasmer::Store;
    use wasmer::TypedFunction;

    pub struct UnitTestHost {
        pub store: Store,
        pub instance: Instance,
        pub memory: Memory,
        pub api_version: Version,
        pub id_of_type: TypedFunction<u32, u32>,
    }

    impl AscHeap for UnitTestHost {
        fn raw_new(&mut self, _bytes: &[u8]) -> Result<u32, AscError> {
            unimplemented!()
        }

        fn read<'a>(
            &self,
            offset: u32,
            buffer: &'a mut [MaybeUninit<u8>],
        ) -> Result<&'a mut [u8], AscError> {
            let store_ref = self.store.as_store_ref();
            let view = self.memory.view(&store_ref);

            view.read_uninit(offset as u64, buffer).map_err(|_| {
                AscError::Plain(format!("Heap access out of bounds. Offset: {}", offset))
            })
        }

        fn read_u32(&self, offset: u32) -> Result<u32, AscError> {
            let mut bytes = [0; 4];
            let store_ref = self.store.as_store_ref();
            let view = self.memory.view(&store_ref);

            view.read(offset as u64, &mut bytes).map_err(|_| {
                AscError::Plain(format!(
                    "Heap access out of bounds. Offset: {} Size: {}",
                    offset, 4
                ))
            })?;
            Ok(u32::from_le_bytes(bytes))
        }

        fn api_version(&self) -> Version {
            self.api_version.clone()
        }

        fn asc_type_id(&mut self, type_id_index: IndexForAscTypeId) -> Result<u32, AscError> {
            self.id_of_type
                .call(&mut self.store.as_store_mut(), type_id_index as u32)
                .map_err(|err| {
                    AscError::Plain(format!(
                        "Failed to get Asc type id for index: {:?}. Trap: {}",
                        type_id_index, err
                    ))
                })
        }
    }
}
