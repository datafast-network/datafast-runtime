use crate::asc::base::AscHeap;
use crate::asc::base::AscPtr;
use crate::asc::base::AscValue;
use crate::asc::errors::AscError;
use crate::asc::native_types::array_buffer::ArrayBuffer;
use crate::impl_asc_type_struct;
use semver::Version;
use std::marker::PhantomData;
use std::mem::size_of;

/// A typed, indexable view of an `ArrayBuffer` of Asc primitives. In Asc it's
/// an abstract class with subclasses for each primitive, for example
/// `Uint8Array` is `TypedArray<u8>`.
/// Also known as `ArrayBufferView`.
/// See https://www.assemblyscript.org/memory.html#arraybufferview-layout
#[repr(C)]
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

impl_asc_type_struct!(
  TypedArray<T>;
    buffer => AscPtr<ArrayBuffer>,
    data_start => u32,
    byte_length => u32,
    ty => PhantomData<T>
);

impl<T: AscValue> TypedArray<T> {
    pub(crate) fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let buffer = ArrayBuffer::new(content, heap.api_version())?;
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
            heap.api_version(),
        )
    }
}
