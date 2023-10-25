mod v0_0_4;
mod v0_0_5;

use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use semver::Version;

pub enum AscString {
    ApiVersion0_0_4(v0_0_4::AscString),
    ApiVersion0_0_5(v0_0_5::AscString),
}

impl AscString {
    pub fn new(content: &[u16], api_version: Version) -> Result<Self, AscError> {
        match api_version {
            version if version <= Version::new(0, 0, 4) => {
                Ok(Self::ApiVersion0_0_4(v0_0_4::AscString::new(content)?))
            }
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::AscString::new(content)?)),
        }
    }

    pub fn content(&self) -> &[u16] {
        match self {
            Self::ApiVersion0_0_4(s) => &s.content,
            Self::ApiVersion0_0_5(s) => &s.content,
        }
    }
}

impl AscIndexId for AscString {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::String;
}

impl AscType for AscString {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        match self {
            Self::ApiVersion0_0_4(s) => s.to_asc_bytes(),
            Self::ApiVersion0_0_5(s) => s.to_asc_bytes(),
        }
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        match api_version {
            version if *version <= Version::new(0, 0, 4) => Ok(Self::ApiVersion0_0_4(
                v0_0_4::AscString::from_asc_bytes(asc_obj, api_version)?,
            )),
            _ => Ok(Self::ApiVersion0_0_5(v0_0_5::AscString::from_asc_bytes(
                asc_obj,
                api_version,
            )?)),
        }
    }

    fn asc_size<H: AscHeap + ?Sized>(ptr: AscPtr<Self>, heap: &H) -> Result<u32, AscError> {
        v0_0_4::AscString::asc_size(AscPtr::new(ptr.wasm_ptr()), heap)
    }

    fn content_len(&self, asc_bytes: &[u8]) -> usize {
        match self {
            Self::ApiVersion0_0_5(s) => s.content_len(asc_bytes),
            _ => unreachable!("Only called for apiVersion >=0.0.5"),
        }
    }
}

impl ToAscObj<AscString> for str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        Ok(AscString::new(
            &self.encode_utf16().collect::<Vec<_>>(),
            heap.api_version(),
        )?)
    }
}

impl ToAscObj<AscString> for &str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        Ok(AscString::new(
            &self.encode_utf16().collect::<Vec<_>>(),
            heap.api_version(),
        )?)
    }
}

impl ToAscObj<AscString> for String {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        self.as_str().to_asc_obj(heap)
    }
}

impl FromAscObj<AscString> for String {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_string: AscString,
        _: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        let mut string =
            String::from_utf16(asc_string.content()).map_err(|e| AscError::Plain(e.to_string()))?;

        // Strip null characters since they are not accepted by Postgres.
        if string.contains('\u{0000}') {
            string = string.replace('\u{0000}', "");
        }
        Ok(string)
    }
}
