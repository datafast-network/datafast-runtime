use crate::asc::base::AscHeap;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::errors::AscError;
use semver::Version;
use std::mem::size_of;
use std::mem::size_of_val;

/// Asc std ArrayBuffer: "a generic, fixed-length raw binary data buffer".
/// See https://github.com/AssemblyScript/assemblyscript/wiki/Memory-Layout-&-Management/86447e88be5aa8ec633eaf5fe364651136d136ab#arrays
pub struct ArrayBuffer {
    pub byte_length: u32,
    // Asc allocators always align at 8 bytes, we already have 4 bytes from
    // `byte_length_size` so with 4 more bytes we align the contents at 8
    // bytes. No Asc type has alignment greater than 8, so the
    // elements in `content` will be aligned for any element type.
    pub padding: [u8; 4],
    // In Asc this slice is layed out inline with the ArrayBuffer.
    pub content: Box<[u8]>,
}

impl ArrayBuffer {
    pub fn new<T: AscType>(values: &[T]) -> Result<Self, AscError> {
        let mut content = Vec::new();
        for value in values {
            let asc_bytes = value.to_asc_bytes()?;
            // An `AscValue` has size equal to alignment, no padding required.
            content.extend(&asc_bytes);
        }

        if content.len() > u32::max_value() as usize {
            return Err(AscError::SizeNotFit);
        }
        Ok(ArrayBuffer {
            byte_length: content.len() as u32,
            padding: [0; 4],
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

        // TODO: This code is preferred as it validates the length of the array.
        // But, some existing subgraphs were found to break when this was added.
        // This needs to be root caused
        /*
        let range = byte_offset..byte_offset + length * size_of::<T>();
        self.content
            .get(range)
            .ok_or_else(|| {
                AscError::from(anyhow::anyhow!("Attempted to read past end of array"))
            })?
            .chunks_exact(size_of::<T>())
            .map(|bytes| T::from_asc_bytes(bytes))
            .collect()
            */
    }
}

impl AscType for ArrayBuffer {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let mut asc_layout: Vec<u8> = Vec::new();

        let byte_length: [u8; 4] = self.byte_length.to_le_bytes();
        asc_layout.extend(byte_length);
        asc_layout.extend(self.padding);
        asc_layout.extend(self.content.iter());

        // Allocate extra capacity to next power of two, as required by asc.
        let header_size = size_of_val(&byte_length) + size_of_val(&self.padding);
        let total_size = self.byte_length as usize + header_size;
        let total_capacity = total_size.next_power_of_two();
        let extra_capacity = total_capacity - total_size;
        asc_layout.extend(std::iter::repeat(0).take(extra_capacity));
        assert_eq!(asc_layout.len(), total_capacity);

        Ok(asc_layout)
    }

    /// The Rust representation of an Asc object as layed out in Asc memory.
    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        // Skip `byte_length` and the padding.
        let content_offset = size_of::<u32>() + 4;
        let byte_length = asc_obj
            .get(..size_of::<u32>())
            .ok_or_else(|| AscError::Plain("Attempted to read past end of array".to_string()))?;
        let content = asc_obj
            .get(content_offset..)
            .ok_or_else(|| AscError::Plain("Attempted to read past end of array".to_string()))?;
        Ok(ArrayBuffer {
            byte_length: u32::from_asc_bytes(byte_length, api_version)?,
            padding: [0; 4],
            content: content.to_vec().into(),
        })
    }

    fn asc_size<H: AscHeap + ?Sized>(ptr: AscPtr<Self>, heap: &H) -> Result<u32, AscError> {
        let byte_length = ptr.read_u32(heap)?;
        let byte_length_size = size_of::<u32>() as u32;
        let padding_size = size_of::<u32>() as u32;
        Ok(byte_length_size + padding_size + byte_length)
    }
}
