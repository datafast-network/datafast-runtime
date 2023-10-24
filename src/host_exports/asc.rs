use crate::asc::base::AscHeap;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::AscError;
use crate::host_exports::Env;
use num_traits::abs;
use std::mem::MaybeUninit;
use wasmer::AsStoreRef;
use wasmer::FunctionEnvMut;

impl AscHeap for FunctionEnvMut<'_, Env> {
    fn raw_new(&mut self, bytes: &[u8]) -> Result<u32, AscError> {
        let require_length = bytes.len() as u64;
        let (env, store) = self.data_and_store_mut();

        let memory = env.memory.as_ref().unwrap();
        let view = memory.view(&store);
        let available_length = view.data_size();

        // For now, not allow increase memory size
        if available_length < require_length {
            return Err(AscError::SizeNotFit);
        }

        // NOTE: write to page's footer
        let ptr = available_length - require_length;
        view.write(ptr, bytes).expect("Failed");

        Ok(ptr as u32)
    }

    fn read<'a>(
        &self,
        offset: u32,
        buffer: &'a mut [MaybeUninit<u8>],
    ) -> Result<&'a mut [u8], AscError> {
        let env = self.data();
        let memory = &env.memory.clone().unwrap();
        let store_ref = self.as_store_ref();
        let view = memory.view(&store_ref);
        view.read_uninit(offset as u64, buffer)
            .map_err(|_| AscError::Plain(format!("Heap access out of bounds. Offset: {}", offset)))
    }

    fn read_u32(&self, offset: u32) -> Result<u32, AscError> {
        let mut bytes = [0; 4];
        let env = self.data();
        let memory = &env.memory.clone().unwrap();
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

    fn asc_type_id(&mut self, type_id_index: IndexForAscTypeId) -> Result<u32, AscError> {
        let (env, mut store_ref) = self.data_and_store_mut();
        env.id_of_type
            .as_ref()
            .unwrap() // Unwrap ok because it's only called on correct apiVersion, look for AscPtr::generate_header
            .call(&mut store_ref, type_id_index as i32)
            .map_err(|trap| {
                AscError::Plain(format!(
                    "Failed to get Asc type id for index: {:?}. Trap: {}",
                    type_id_index,
                    trap.to_string()
                ))
            })
            .map(|result| abs(result) as u32)
    }
}
