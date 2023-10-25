mod v0_0_4;
mod v0_0_5;

use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::AscError;
use semver::Version;

pub enum ArrayBuffer {
    ApiVersion0_0_4(v0_0_4::ArrayBuffer),
    ApiVersion0_0_5(v0_0_5::ArrayBuffer),
}

impl ArrayBuffer {
    pub(crate) fn new<T: AscType>(values: &[T], api_version: Version) -> Result<Self, AscError> {
        match api_version {
            version if version <= Version::new(0, 0, 4) => {
                Ok(Self::ApiVersion0_0_4(v0_0_4::ArrayBuffer::new(values)?))
            }
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::ArrayBuffer::new(values)?)),
        }
    }
}

impl AscType for ArrayBuffer {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        match self {
            Self::ApiVersion0_0_4(a) => a.to_asc_bytes(),
            Self::ApiVersion0_0_5(a) => a.to_asc_bytes(),
        }
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        match api_version {
            version if *version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::ArrayBuffer::from_asc_bytes(asc_obj, api_version)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::ArrayBuffer::from_asc_bytes(
                asc_obj,
                api_version,
            )?)),
        }
    }

    fn asc_size<H: AscHeap + ?Sized>(ptr: AscPtr<Self>, heap: &H) -> Result<u32, AscError> {
        v0_0_4::ArrayBuffer::asc_size(AscPtr::new(ptr.wasm_ptr()), heap)
    }

    fn content_len(&self, asc_bytes: &[u8]) -> usize {
        match self {
            Self::ApiVersion0_0_5(a) => a.content_len(asc_bytes),
            _ => unreachable!("Only called for apiVersion >=0.0.5"),
        }
    }
}

impl AscIndexId for ArrayBuffer {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayBuffer;
}
