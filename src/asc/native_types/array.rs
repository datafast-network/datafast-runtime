use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscValue;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::AscError;
use crate::impl_asc_type_struct;

use super::array_buffer::ArrayBuffer;
use super::string::AscString;
use super::Uint8Array;

use std::marker::PhantomData;

#[repr(C)]
pub struct Array<T> {
    // #data -> Backing buffer reference
    buffer: AscPtr<ArrayBuffer>,
    // #dataStart -> Start of the data within #data
    buffer_data_start: u32,
    // #dataLength -> Length of the data from #dataStart
    buffer_data_length: u32,
    // #length -> Mutable length of the data the user is interested in
    length: i32,
    // Not included in memory layout, it's just for typings
    ty: PhantomData<T>,
}

impl_asc_type_struct!(
    Array<T>;
    buffer => AscPtr<ArrayBuffer>,
    buffer_data_start => u32,
    buffer_data_length => u32,
    length => i32,
    ty => PhantomData<T>
);

impl AscIndexId for Array<bool> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayBool;
}

impl AscIndexId for Array<Uint8Array> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayUint8Array;
}

impl<T: AscValue> Array<T> {
    pub fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let arr_buffer = ArrayBuffer::new(content)?;
        let buffer = AscPtr::alloc_obj(arr_buffer, heap)?;
        let buffer_data_length = buffer.read_len(heap)?;
        Ok(Array {
            buffer: AscPtr::new(buffer.wasm_ptr()),
            buffer_data_start: buffer.wasm_ptr(),
            buffer_data_length,
            length: content.len() as i32,
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
        // - The second (self.buffer_data_start) points to where in this ArrayBuffer the data starts.
        // So this is basically getting the offset.
        // Related docs: https://www.assemblyscript.org/memory.html#arraybufferview-layout
        let buffer_data_start_with_offset = self
            .buffer_data_start
            .checked_sub(self.buffer.wasm_ptr())
            .ok_or_else(|| {
                AscError::Plain(format!(
                    "Subtract overflow on pointer: {}",
                    self.buffer_data_start
                ))
            })?;

        self.buffer
            .read_ptr(heap)?
            .get(buffer_data_start_with_offset, self.length as u32)
    }
}

impl AscIndexId for Array<AscPtr<AscString>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayString;
}

impl AscIndexId for Array<u8> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayU8;
}

impl AscIndexId for Array<u16> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayU16;
}

impl AscIndexId for Array<u32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayU32;
}

impl AscIndexId for Array<u64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayU64;
}

impl AscIndexId for Array<i8> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayI8;
}

impl AscIndexId for Array<i16> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayI16;
}

impl AscIndexId for Array<i32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayI32;
}

impl AscIndexId for Array<i64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayI64;
}

impl AscIndexId for Array<f32> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayF32;
}

impl AscIndexId for Array<f64> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayF64;
}
