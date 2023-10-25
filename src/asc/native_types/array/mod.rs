pub mod v0_0_4;
pub mod v0_0_5;

/// Wrapper of Array for multiple AssemblyScript versions.
/// It just delegates its method calls to the correct mappings apiVersion.
pub enum Array<T> {
    ApiVersion0_0_4(v0_0_4::Array<T>),
    ApiVersion0_0_5(v0_0_5::Array<T>),
}

impl<T: AscValue> Array<T> {
    pub fn new<H: AscHeap + ?Sized>(
        content: &[T],
        heap: &mut H,
        gas: &GasCounter,
    ) -> Result<Self, HostExportError> {
        match heap.api_version() {
            version if version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::Array::new(content, heap, gas)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::Array::new(
                content, heap, gas,
            )?)),
        }
    }

    pub(crate) fn to_vec<H: AscHeap + ?Sized>(
        &self,
        heap: &H,
        gas: &GasCounter,
    ) -> Result<Vec<T>, DeterministicHostError> {
        match self {
            Self::ApiVersion0_0_4(a) => a.to_vec(heap, gas),
            Self::ApiVersion0_0_5(a) => a.to_vec(heap, gas),
        }
    }
}

impl<T> AscType for Array<T> {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, DeterministicHostError> {
        match self {
            Self::ApiVersion0_0_4(a) => a.to_asc_bytes(),
            Self::ApiVersion0_0_5(a) => a.to_asc_bytes(),
        }
    }

    fn from_asc_bytes(
        asc_obj: &[u8],
        api_version: &Version,
    ) -> Result<Self, DeterministicHostError> {
        match api_version {
            version if *version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::Array::from_asc_bytes(asc_obj, api_version)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::Array::from_asc_bytes(
                asc_obj,
                api_version,
            )?)),
        }
    }
}

impl AscIndexId for Array<bool> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayBool;
}

impl AscIndexId for Array<Uint8Array> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayUint8Array;
}

impl AscIndexId for Array<AscPtr<AscEnum<EthereumValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEthereumValue;
}

impl AscIndexId for Array<AscPtr<AscEnum<StoreValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayStoreValue;
}

impl AscIndexId for Array<AscPtr<AscEnum<JsonValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayJsonValue;
}

impl AscIndexId for Array<AscPtr<AscString>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayString;
}

impl AscIndexId for Array<AscPtr<AscTypedMapEntry<AscString, AscEnum<JsonValueKind>>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId =
        IndexForAscTypeId::ArrayTypedMapEntryStringJsonValue;
}

impl AscIndexId for Array<AscPtr<AscTypedMapEntry<AscString, AscEnum<StoreValueKind>>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId =
        IndexForAscTypeId::ArrayTypedMapEntryStringStoreValue;
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

impl AscIndexId for Array<AscPtr<AscBigDecimal>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayBigDecimal;
}
