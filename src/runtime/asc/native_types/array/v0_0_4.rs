use crate::errors::AscError;
use crate::impl_asc_type_struct;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscValue;
use crate::runtime::asc::native_types::array_buffer;
use crate::runtime::asc::native_types::array_buffer::v0_0_4::ArrayBuffer;
use semver::Version;
use std::marker::PhantomData;

#[repr(C)]
pub struct Array<T> {
    buffer: AscPtr<ArrayBuffer>,
    length: u32,
    ty: PhantomData<T>,
}

impl_asc_type_struct!(
    Array<T>;
    buffer => AscPtr<ArrayBuffer>,
    length => u32,
    ty => PhantomData<T>
);

impl<T: AscValue> Array<T> {
    pub fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        let arr_buffer = array_buffer::ArrayBuffer::new(content, heap.api_version())?;
        let arr_buffer_ptr = AscPtr::alloc_obj(arr_buffer, heap)?;
        Ok(Array {
            buffer: AscPtr::new(arr_buffer_ptr.wasm_ptr()),
            // If this cast would overflow, the above line has already panicked.
            length: content.len() as u32,
            ty: PhantomData,
        })
    }

    pub fn to_vec<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<Vec<T>, AscError> {
        self.buffer
            .read_ptr(heap)?
            .get(0, self.length, heap.api_version())
    }
}
