use crate::impl_asc_type_struct;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscValue;
use crate::runtime::asc::errors::AscError;
use crate::runtime::asc::native_types::array_buffer;
use crate::runtime::asc::native_types::array_buffer::v0_0_5::ArrayBuffer;
use semver::Version;
use std::marker::PhantomData;

/// Growable array backed by an `ArrayBuffer`.
/// See https://www.assemblyscript.org/memory.html#array-layout
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

impl<T: AscValue> Array<T> {
    pub fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let arr_buffer = array_buffer::ArrayBuffer::new(content, heap.api_version())?;
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

    pub fn to_vec<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<Vec<T>, AscError> {
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

        self.buffer.read_ptr(heap)?.get(
            buffer_data_start_with_offset,
            self.length as u32,
            heap.api_version(),
        )
    }
}
