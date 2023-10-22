use super::array::Array;
use super::json::JsonValueKind;
use super::store::StoreValueKind;

use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::AscValue;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::errors::DeterministicHostError;
use crate::impl_asc_type_struct;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct EnumPayload(pub u64);

impl AscType for EnumPayload {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, DeterministicHostError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, DeterministicHostError> {
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

impl_asc_type_struct!(
    AscEnum<D: AscValue>;
    kind => D,
    _padding => u32,
    payload => EnumPayload
);

impl AscIndexId for AscEnum<StoreValueKind> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::StoreValue;
}

impl AscIndexId for AscEnum<JsonValueKind> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::JsonValue;
}

pub type AscEnumArray<D> = AscPtr<Array<AscPtr<AscEnum<D>>>>;
