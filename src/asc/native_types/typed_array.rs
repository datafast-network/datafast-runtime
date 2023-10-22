use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscValue;
use crate::asc::base::FromAscObj;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::DeterministicHostError;
use crate::asc::errors::HostExportError;
use crate::impl_asc_type_struct;

use super::array_buffer::ArrayBuffer;

use std::marker::PhantomData;
use std::mem::size_of;

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
    pub(crate) fn new<H: AscHeap + ?Sized>(
        content: &[T],
        heap: &mut H,
    ) -> Result<Self, HostExportError> {
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

    pub(crate) fn to_vec<H: AscHeap + ?Sized>(
        &self,
        heap: &H,
    ) -> Result<Vec<T>, DeterministicHostError> {
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
                DeterministicHostError::from(anyhow::anyhow!(
                    "Subtract overflow on pointer: {}",
                    self.data_start
                ))
            })?;

        self.buffer.read_ptr(heap)?.get(
            data_start_with_offset,
            self.byte_length / size_of::<T>() as u32,
        )
    }
}

impl<T: AscValue> FromAscObj<TypedArray<T>> for Vec<T> {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: TypedArray<T>,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, DeterministicHostError> {
        typed_array.to_vec(heap)
    }
}

impl<T: AscValue> ToAscObj<TypedArray<T>> for [T] {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<TypedArray<T>, HostExportError> {
        TypedArray::new(self, heap)
    }
}

impl<T: AscValue + Send + Sync, const LEN: usize> FromAscObj<TypedArray<T>> for [T; LEN] {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: TypedArray<T>,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, DeterministicHostError> {
        let v = typed_array.to_vec(heap)?;
        let array = <[T; LEN]>::try_from(v).map_err(|v| {
            DeterministicHostError::from(anyhow::anyhow!(
                "expected array of length {}, found length {}",
                LEN,
                v.len()
            ))
        })?;
        Ok(array)
    }
}
