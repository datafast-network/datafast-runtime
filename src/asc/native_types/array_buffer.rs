use crate::asc::base::AscIndexId;
use crate::asc::base::AscType;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::AscError;
use semver::Version;
use std::mem::size_of;

pub struct ArrayBuffer {
    // Not included in memory layout
    pub byte_length: u32,
    // #data
    pub content: Box<[u8]>,
}

impl ArrayBuffer {
    pub fn new<T: AscType>(values: &[T]) -> Result<Self, AscError> {
        let mut content = Vec::new();
        for value in values {
            let asc_bytes = value.to_asc_bytes()?;
            content.extend(&asc_bytes);
        }

        if content.len() > u32::MAX as usize {
            return Err(AscError::Plain(
                "slice cannot fit in WASM memory".to_string(),
            ));
        }
        Ok(ArrayBuffer {
            byte_length: content.len() as u32,
            content: content.into(),
        })
    }

    /// Read `length` elements of type `T` starting at `byte_offset`.
    ///
    /// Panics if that tries to read beyond the length of `self.content`.
    pub fn get<T: AscType>(&self, byte_offset: u32, length: u32) -> Result<Vec<T>, AscError> {
        let length = length as usize;
        let byte_offset = byte_offset as usize;

        self.content[byte_offset..]
            .chunks(size_of::<T>())
            .take(length)
            .map(|asc_obj| T::from_asc_bytes(asc_obj))
            .collect()
    }
}

impl AscIndexId for ArrayBuffer {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayBuffer;
}

impl AscType for ArrayBuffer {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        // let in_memory_byte_count = size_of::<Self>();
        // let mut bytes = Vec::with_capacity(in_memory_byte_count);
        //
        // let mut offset = 0;
        // // max field alignment will also be struct alignment which we need to pad the end
        // let mut max_align = 0;

        Ok(vec![])
    }

    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        Ok(ArrayBuffer {
            byte_length: asc_obj.len() as u32,
            content: asc_obj.to_vec().into(),
        })
    }

    fn content_len(&self, _asc_bytes: &[u8]) -> usize {
        self.byte_length as usize // without extra_capacity
    }
}
