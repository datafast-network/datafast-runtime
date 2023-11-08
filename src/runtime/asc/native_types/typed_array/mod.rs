mod v0_0_4;
mod v0_0_5;

use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscType;
use crate::runtime::asc::base::AscValue;
use crate::runtime::asc::base::IndexForAscTypeId;
use crate::runtime::asc::errors::AscError;
use semver::Version;

pub enum TypedArray<T> {
    ApiVersion0_0_4(v0_0_4::TypedArray<T>),
    ApiVersion0_0_5(v0_0_5::TypedArray<T>),
}

impl<T: AscValue> TypedArray<T> {
    pub fn new<H: AscHeap + ?Sized>(content: &[T], heap: &mut H) -> Result<Self, AscError> {
        match heap.api_version() {
            version if version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::TypedArray::new(content, heap)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::TypedArray::new(
                content, heap,
            )?)),
        }
    }

    pub fn to_vec<H: AscHeap + ?Sized>(&self, heap: &H) -> Result<Vec<T>, AscError> {
        match self {
            Self::ApiVersion0_0_4(t) => t.to_vec(heap),
            Self::ApiVersion0_0_5(t) => t.to_vec(heap),
        }
    }
}

impl<T> AscType for TypedArray<T> {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        match self {
            Self::ApiVersion0_0_4(t) => t.to_asc_bytes(),
            Self::ApiVersion0_0_5(t) => t.to_asc_bytes(),
        }
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        match api_version {
            version if *version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::TypedArray::from_asc_bytes(asc_obj, api_version)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::TypedArray::from_asc_bytes(
                asc_obj,
                api_version,
            )?)),
        }
    }
}

pub type Uint8Array = TypedArray<u8>;

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
