use super::asc_base::AscHeap;
use super::asc_base::AscIndexId;
use super::asc_base::AscPtr;
use super::asc_base::AscType;
use super::asc_base::AscValue;
use super::asc_base::IndexForAscTypeId;
use super::asc_base::HEADER_SIZE;
use super::errors::AscError;

use std::marker::PhantomData;
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

        if content.len() > u32::max_value() as usize {
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
        let mut asc_layout: Vec<u8> = Vec::new();

        asc_layout.extend(self.content.iter());

        // Allocate extra capacity to next power of two, as required by asc.
        let total_size = self.byte_length as usize + HEADER_SIZE;
        let total_capacity = total_size.next_power_of_two();
        let extra_capacity = total_capacity - total_size;
        asc_layout.extend(std::iter::repeat(0).take(extra_capacity));

        Ok(asc_layout)
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

#[repr(C)]
// #[derive(AscType)] # NOTE: fix
pub struct TypedArray<T> {
    // #data -> Backing buffer reference
    pub buffer: AscPtr<ArrayBuffer>,
    // #dataStart -> Start within the #data
    data_start: u32,
    // #dataLength -> Length of the data from #dataStart
    byte_length: u32,
    // Not included in memory layout, it's just for typings
    ty: PhantomData<T>,
}

// impl<T> AscIndexId for TypedArray<T> {
//     // NOTE: not sure if this is critical!
//     const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Uint8Array;
// }

impl AscIndexId for TypedArray<i8> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Int8Array;
}

impl AscIndexId for TypedArray<i16> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Int16Array;
}

impl AscIndexId for TypedArray<i32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Int32Array;
}

impl AscIndexId for TypedArray<i64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Int64Array;
}

impl AscIndexId for TypedArray<u8> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Uint8Array;
}

impl AscIndexId for TypedArray<u16> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Uint16Array;
}

impl AscIndexId for TypedArray<u32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Uint32Array;
}

impl AscIndexId for TypedArray<u64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Uint64Array;
}

impl AscIndexId for TypedArray<f32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Float32Array;
}

impl AscIndexId for TypedArray<f64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Float64Array;
}

impl<T: AscValue> TypedArray<T> {
    pub(crate) fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let buffer = ArrayBuffer::new(content)?;
        let byte_length = content.len() as u32;
        let ptr = AscPtr::alloc_obj(buffer, heap)?;
        Ok(TypedArray {
            buffer: AscPtr::new(ptr.wasm_ptr()), // new AscPtr necessary to convert type parameter
            data_start: ptr.wasm_ptr(),
            byte_length,
            ty: PhantomData,
        })
    }

    pub(crate) fn to_vec<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<Vec<T>, AscError> {
        // We're trying to read the pointer below, we should check it's
        // not null before using it.
        self.buffer.check_is_not_null()?;

        // This subtraction is needed because on the ArrayBufferView memory layout
        // there are two pointers to the data.
        // - The first (self.buffer) points to the related ArrayBuffer.
        // - The second (self.data_start) points to where in this ArrayBuffer the data starts.
        // So this is basically getting the offset.
        // Related docs: https://www.assemblyscript.org/memory.html#arraybufferview-layout
        let data_start_with_offset = self
            .data_start
            .checked_sub(self.buffer.wasm_ptr())
            .ok_or_else(|| {
                AscError::Plain(format!("Subtract overflow on pointer: {}", self.data_start))
            })?;

        self.buffer.read_ptr(heap)?.get(
            data_start_with_offset,
            self.byte_length / size_of::<T>() as u32,
        )
    }
}

pub type Uint8Array = TypedArray<u8>;
