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
use typed_array::TypedArray;

use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscValue;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::impl_asc_type_struct;

pub type Uint8Array = TypedArray<u8>;

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
