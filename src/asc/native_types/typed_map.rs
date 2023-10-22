use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::HostExportError;
use crate::impl_asc_type_struct;

use super::array::Array;
use super::json::JsonValueKind;
use super::r#enum::AscEnum;
use super::store::StoreValueKind;
use super::string::AscString;

#[repr(C)]
pub struct AscTypedMapEntry<K, V> {
    pub key: AscPtr<K>,
    pub value: AscPtr<V>,
}

impl_asc_type_struct!(
    AscTypedMapEntry<K, V>;
    key => AscPtr<K>,
    value => AscPtr<V>
);

impl AscIndexId for AscTypedMapEntry<AscString, AscEnum<StoreValueKind>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TypedMapEntryStringStoreValue;
}

impl AscIndexId for AscTypedMapEntry<AscString, AscEnum<JsonValueKind>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TypedMapEntryStringJsonValue;
}

pub type AscTypedMapEntryArray<K, V> = Array<AscPtr<AscTypedMapEntry<K, V>>>;

#[repr(C)]
pub struct AscTypedMap<K, V> {
    pub entries: AscPtr<AscTypedMapEntryArray<K, V>>,
}

impl_asc_type_struct!(
    AscTypedMap<K, V>;
    entries => AscPtr<AscTypedMapEntryArray<K, V>>
);

pub type AscEntity = AscTypedMap<AscString, AscEnum<StoreValueKind>>;

impl AscIndexId for AscTypedMap<AscString, AscEnum<StoreValueKind>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TypedMapStringStoreValue;
}

impl AscIndexId for Array<AscPtr<AscEntity>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayTypedMapStringStoreValue;
}

impl AscIndexId for AscTypedMap<AscString, AscEnum<JsonValueKind>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TypedMapStringJsonValue;
}

impl AscIndexId for AscTypedMap<AscString, AscTypedMap<AscString, AscEnum<JsonValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId =
        IndexForAscTypeId::TypedMapStringTypedMapStringJsonValue;
}

impl<K: AscType + AscIndexId, V: AscType + AscIndexId, T: ToAscObj<K>, U: ToAscObj<V>>
    ToAscObj<AscTypedMapEntry<K, V>> for (T, U)
{
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscTypedMapEntry<K, V>, HostExportError> {
        Ok(AscTypedMapEntry {
            key: asc_new(heap, &self.0)?,
            value: asc_new(heap, &self.1)?,
        })
    }
}
