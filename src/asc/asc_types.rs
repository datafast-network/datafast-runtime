use super::asc_base::AscHeap;
use super::asc_base::AscIndexId;
use super::asc_base::AscPtr;
use super::asc_base::AscType;
use super::asc_base::AscValue;
use super::asc_base::IndexForAscTypeId;
use super::errors::AscError;

use crate::asc::asc_base::ToAscObj;
use crate::impl_asc_type_struct;
use std::marker::PhantomData;
use std::mem::{size_of, size_of_val};

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

pub struct AscString {
    // Not included in memory layout
    // In number of UTF-16 code units (2 bytes each).
    byte_length: u32,
    // #data
    // The sequence of UTF-16LE code units that form the string.
    pub content: Box<[u16]>,
}

impl AscIndexId for AscString {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::String;
}

impl AscString {
    pub fn new(content: &[u16]) -> Result<Self, AscError> {
        if size_of_val(content) > u32::MAX as usize {
            return Err(AscError::Plain(
                "string cannot fit in WASM memory".to_string(),
            ));
        }

        Ok(AscString {
            byte_length: content.len() as u32,
            content: content.into(),
        })
    }
}

impl AscType for AscString {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let mut content: Vec<u8> = Vec::new();

        // Write the code points, in little-endian (LE) order.
        for &code_unit in self.content.iter() {
            let low_byte = code_unit as u8;
            let high_byte = (code_unit >> 8) as u8;
            content.push(low_byte);
            content.push(high_byte);
        }

        let header_size = 20;
        let total_size = (self.byte_length as usize * 2) + header_size;
        let total_capacity = total_size.next_power_of_two();
        let extra_capacity = total_capacity - total_size;
        content.extend(std::iter::repeat(0).take(extra_capacity));

        Ok(content)
    }

    /// The Rust representation of an Asc object as layed out in Asc memory.
    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        // UTF-16 (used in assemblyscript) always uses one
        // pair of bytes per code unit.
        // https://mathiasbynens.be/notes/javascript-encoding
        // UTF-16 (16-bit Unicode Transformation Format) is an
        // extension of UCS-2 that allows representing code points
        // outside the BMP. It produces a variable-length result
        // of either one or two 16-bit code units per code point.
        // This way, it can encode code points in the range from 0
        // to 0x10FFFF.

        let mut content = Vec::new();
        for pair in asc_obj.chunks(2) {
            let code_point_bytes = [
                pair[0],
                *pair.get(1).ok_or_else(|| {
                    AscError::Plain(
                        "Attempted to read past end of string content bytes chunk".to_string(),
                    )
                })?,
            ];
            let code_point = u16::from_le_bytes(code_point_bytes);
            content.push(code_point);
        }
        AscString::new(&content)
    }

    fn content_len(&self, _asc_bytes: &[u8]) -> usize {
        self.byte_length as usize * 2 // without extra_capacity, and times 2 because the content is measured in u8s
    }
}

impl ToAscObj<AscString> for str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        Ok(AscString::new(&self.encode_utf16().collect::<Vec<_>>())?)
    }
}

impl ToAscObj<AscString> for &str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        Ok(AscString::new(&self.encode_utf16().collect::<Vec<_>>())?)
    }
}

impl ToAscObj<AscString> for String {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        self.as_str().to_asc_obj(heap)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct EnumPayload(pub u64);

impl AscType for EnumPayload {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        Ok(EnumPayload(u64::from_asc_bytes(asc_obj)?))
    }
}

impl From<EnumPayload> for i32 {
    fn from(payload: EnumPayload) -> i32 {
        payload.0 as i32
    }
}

impl From<EnumPayload> for f64 {
    fn from(payload: EnumPayload) -> f64 {
        f64::from_bits(payload.0)
    }
}

impl From<EnumPayload> for i64 {
    fn from(payload: EnumPayload) -> i64 {
        payload.0 as i64
    }
}

impl From<EnumPayload> for bool {
    fn from(payload: EnumPayload) -> bool {
        payload.0 != 0
    }
}

impl From<i32> for EnumPayload {
    fn from(x: i32) -> EnumPayload {
        EnumPayload(x as u64)
    }
}

impl From<f64> for EnumPayload {
    fn from(x: f64) -> EnumPayload {
        EnumPayload(x.to_bits())
    }
}

impl From<bool> for EnumPayload {
    fn from(b: bool) -> EnumPayload {
        EnumPayload(b.into())
    }
}

impl From<i64> for EnumPayload {
    fn from(x: i64) -> EnumPayload {
        EnumPayload(x as u64)
    }
}

impl<C> From<EnumPayload> for AscPtr<C> {
    fn from(payload: EnumPayload) -> Self {
        AscPtr::new(payload.0 as u32)
    }
}

impl<C> From<AscPtr<C>> for EnumPayload {
    fn from(x: AscPtr<C>) -> EnumPayload {
        EnumPayload(x.wasm_ptr() as u64)
    }
}

#[repr(C)]
pub struct AscEnum<D: AscValue> {
    pub kind: D,
    pub _padding: u32, // Make padding explicit.
    pub payload: EnumPayload,
}

pub type AscEnumArray<D> = AscPtr<Array<AscPtr<AscEnum<D>>>>;
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
