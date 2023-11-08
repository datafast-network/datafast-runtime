use crate::errors::AscError;
use crate::impl_asc_type_struct;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscValue;
use crate::runtime::asc::native_types::array_buffer;
use crate::runtime::asc::native_types::array_buffer::v0_0_4::ArrayBuffer;
use semver::Version;
use std::marker::PhantomData;
use std::mem::size_of;

/// A typed, indexable view of an `ArrayBuffer` of Asc primitives. In Asc it's
/// an abstract class with subclasses for each primitive, for example
/// `Uint8Array` is `TypedArray<u8>`.
///  See https://github.com/AssemblyScript/assemblyscript/wiki/Memory-Layout-&-Management/86447e88be5aa8ec633eaf5fe364651136d136ab#arrays
#[repr(C)]
pub struct TypedArray<T> {
    pub buffer: AscPtr<ArrayBuffer>,
    /// Byte position in `buffer` of the array start.
    byte_offset: u32,
    byte_length: u32,
    ty: PhantomData<T>,
}

impl_asc_type_struct!(
  TypedArray<T>;
    buffer => AscPtr<ArrayBuffer>,
    byte_offset => u32,
    byte_length => u32,
    ty => PhantomData<T>
);

impl<T: AscValue> TypedArray<T> {
    pub fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let buffer = array_buffer::ArrayBuffer::new(content, heap.api_version())?;
        let buffer_byte_length = if let array_buffer::ArrayBuffer::ApiVersion0_0_4(ref a) = buffer {
            a.byte_length
        } else {
            unreachable!("Only the correct ArrayBuffer will be constructed")
        };
        let ptr = AscPtr::alloc_obj(buffer, heap)?;
        Ok(TypedArray {
            byte_length: buffer_byte_length,
            buffer: AscPtr::new(ptr.wasm_ptr()),
            byte_offset: 0,
            ty: PhantomData,
        })
    }

    pub fn to_vec<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<Vec<T>, AscError> {
        self.buffer.read_ptr(heap)?.get(
            self.byte_offset,
            self.byte_length / size_of::<T>() as u32,
            heap.api_version(),
        )
    }
}
