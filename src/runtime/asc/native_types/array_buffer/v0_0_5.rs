use crate::errors::AscError;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::HEADER_SIZE;
use semver::Version;
use std::mem::size_of;

/// Similar as JS ArrayBuffer, "a generic, fixed-length raw binary data buffer".
/// See https://www.assemblyscript.org/memory.html#arraybuffer-layout
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

        if content.len() > u32::max_value() as usize {
            return Err(AscError::SizeNotFit);
        }
        Ok(ArrayBuffer {
            byte_length: content.len() as u32,
            content: content.into(),
        })
    }

    /// Read `length` elements of type `T` starting at `byte_offset`.
    ///
    /// Panics if that tries to read beyond the length of `self.content`.
    pub fn get<T: AscType>(
        &self,
        byte_offset: u32,
        length: u32,
        api_version: Version,
    ) -> Result<Vec<T>, AscError> {
        let length = length as usize;
        let byte_offset = byte_offset as usize;

        self.content[byte_offset..]
            .chunks(size_of::<T>())
            .take(length)
            .map(|asc_obj| T::from_asc_bytes(asc_obj, &api_version))
            .collect()
    }
}

impl AscType for ArrayBuffer {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let mut asc_layout: Vec<u8> = Vec::new();

        asc_layout.extend(self.content.iter());

        // Allocate extra capacity to next power of two, as required by asc.
        let total_size = self.byte_length as usize + HEADER_SIZE;
        let total_capacity = total_size.next_power_of_two();
        let extra_capacity = total_capacity - total_size;
        asc_layout.extend(std::iter::repeat(0).take(extra_capacity));

        Ok(asc_layout)
    }

    fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, AscError> {
        Ok(ArrayBuffer {
            byte_length: asc_obj.len() as u32,
            content: asc_obj.to_vec().into(),
        })
    }

    fn content_len(&self, _asc_bytes: &[u8]) -> usize {
        self.byte_length as usize // without extra_capacity
    }
}
