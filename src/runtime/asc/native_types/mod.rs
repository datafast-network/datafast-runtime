pub mod array;
pub mod array_buffer;
pub mod r#enum;
pub mod json;
pub mod store;
pub mod string;
pub mod typed_array;
pub mod typed_map;

use json::AscJson;
use json::JsonValueKind;
use r#enum::AscEnum;
use semver::Version;
use typed_array::TypedArray;

use crate::impl_asc_type_struct;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscValue;
use crate::runtime::asc::base::FromAscObj;
use crate::runtime::asc::base::IndexForAscTypeId;
use crate::runtime::asc::base::ToAscObj;
use crate::runtime::asc::errors::AscError;

pub type Uint8Array = TypedArray<u8>;

pub type AscH160 = Uint8Array;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct AscWrapped<V: AscValue> {
    pub inner: V,
}

impl_asc_type_struct!(
    AscWrapped<V: AscValue>;
    inner => V
);

impl AscIndexId for AscWrapped<AscPtr<AscJson>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::WrappedTypedMapStringJsonValue;
}

impl AscIndexId for AscWrapped<bool> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::WrappedBool;
}

impl AscIndexId for AscWrapped<AscPtr<AscEnum<JsonValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::WrappedJsonValue;
}

impl<T: AscValue> ToAscObj<AscWrapped<T>> for AscWrapped<T> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, _heap: &mut H) -> Result<AscWrapped<T>, AscError> {
        Ok(*self)
    }
}

impl<T: AscValue> FromAscObj<AscWrapped<T>> for T {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_obj: AscWrapped<T>,
        _heap: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        Ok(asc_obj.inner)
    }
}

pub struct Bytes<'a>(pub &'a Vec<u8>);

impl ToAscObj<Uint8Array> for Bytes<'_> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<Uint8Array, AscError> {
        self.0.to_asc_obj(heap)
    }
}
#[repr(C)]
pub struct AscResult<V: AscValue, E: AscValue> {
    pub value: AscPtr<AscWrapped<V>>,
    pub error: AscPtr<AscWrapped<E>>,
}

impl_asc_type_struct!(
    AscResult<V: AscValue, E: AscValue>;
    value => AscPtr<AscWrapped<V>>,
    error => AscPtr<AscWrapped<E>>
);

impl AscIndexId for AscResult<AscPtr<AscJson>, bool> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId =
        IndexForAscTypeId::ResultTypedMapStringJsonValueBool;
}

impl AscIndexId for AscResult<AscPtr<AscEnum<JsonValueKind>>, bool> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ResultJsonValueBool;
}
