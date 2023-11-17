use super::base::asc_get;
use super::base::asc_new;
use super::base::AscHeap;
use super::base::AscIndexId;
use super::base::AscPtr;
use super::base::AscType;
use super::base::AscValue;
use super::base::FromAscObj;
use super::base::ToAscObj;
use super::bignumber::AscBigDecimal;
use super::bignumber::AscBigInt;
use super::native_types::array::Array;
use super::native_types::json::AscJson;
use super::native_types::json::JsonValueKind;
use super::native_types::r#enum::AscEnum;
use super::native_types::r#enum::AscEnumArray;
use super::native_types::r#enum::EnumPayload;
use super::native_types::store::StoreValueKind;
use super::native_types::store::Value;
use super::native_types::string::AscString;
use super::native_types::typed_array::TypedArray;
use super::native_types::typed_array::Uint8Array;
use super::native_types::typed_map::AscEntity;
use super::native_types::typed_map::AscTypedMap;
use super::native_types::typed_map::AscTypedMapEntry;
use crate::errors::AscError;
use crate::runtime::bignumber::bigint::BigInt;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FromIterator;
use crate::messages::RawEntity;

/// Implementations of `ToAscObj` and `FromAscObj` for Rust types.
/// Standard Rust types go in `mod.rs` and external types in `external.rs`.

impl<T: AscValue> ToAscObj<TypedArray<T>> for [T] {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<TypedArray<T>, AscError> {
        TypedArray::new(self, heap)
    }
}

impl<T: AscValue> FromAscObj<TypedArray<T>> for Vec<T> {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: TypedArray<T>,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        typed_array.to_vec(heap)
    }
}

impl<T: AscValue + Send + Sync, const LEN: usize> FromAscObj<TypedArray<T>> for [T; LEN] {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: TypedArray<T>,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        let v = typed_array.to_vec(heap)?;
        let array = <[T; LEN]>::try_from(v).map_err(|v| {
            AscError::Plain(format!(
                "expected array of length {}, found length {}",
                LEN,
                v.len()
            ))
        })?;
        Ok(array)
    }
}

impl ToAscObj<AscString> for str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        AscString::new(&self.encode_utf16().collect::<Vec<_>>(), heap.api_version())
    }
}

impl ToAscObj<AscString> for &str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        AscString::new(&self.encode_utf16().collect::<Vec<_>>(), heap.api_version())
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

impl<C: AscType + AscIndexId, T: ToAscObj<C>> ToAscObj<Array<AscPtr<C>>> for [T] {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<Array<AscPtr<C>>, AscError> {
        let content: Result<Vec<_>, _> = self.iter().map(|x| asc_new(heap, x)).collect();
        let content = content?;
        Array::new(&content, heap)
    }
}

impl<C: AscType + AscIndexId, T: FromAscObj<C>> FromAscObj<Array<AscPtr<C>>> for Vec<T> {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        array: Array<AscPtr<C>>,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        array
            .to_vec(heap)?
            .into_iter()
            .map(|x| asc_get(heap, x, depth))
            .collect()
    }
}

impl<K: AscType + AscIndexId, V: AscType + AscIndexId, T: FromAscObj<K>, U: FromAscObj<V>>
    FromAscObj<AscTypedMapEntry<K, V>> for (T, U)
{
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_entry: AscTypedMapEntry<K, V>,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok((
            asc_get(heap, asc_entry.key, depth)?,
            asc_get(heap, asc_entry.value, depth)?,
        ))
    }
}

impl<K: AscType + AscIndexId, V: AscType + AscIndexId, T: ToAscObj<K>, U: ToAscObj<V>>
    ToAscObj<AscTypedMapEntry<K, V>> for (T, U)
{
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscTypedMapEntry<K, V>, AscError> {
        Ok(AscTypedMapEntry {
            key: asc_new(heap, &self.0)?,
            value: asc_new(heap, &self.1)?,
        })
    }
}

impl<
        K: AscType + AscIndexId,
        V: AscType + AscIndexId,
        T: FromAscObj<K> + Hash + Eq,
        U: FromAscObj<V>,
    > FromAscObj<AscTypedMap<K, V>> for HashMap<T, U>
where
    Array<AscPtr<AscTypedMapEntry<K, V>>>: AscIndexId,
    AscTypedMapEntry<K, V>: AscIndexId,
{
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_map: AscTypedMap<K, V>,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let entries: Vec<(T, U)> = asc_get(heap, asc_map.entries, depth)?;
        Ok(HashMap::from_iter(entries.into_iter()))
    }
}

impl FromAscObj<AscEnum<StoreValueKind>> for Value {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_enum: AscEnum<StoreValueKind>,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let payload = asc_enum.payload;
        Ok(match asc_enum.kind {
            StoreValueKind::String => {
                let ptr: AscPtr<AscString> = AscPtr::from(payload);
                Value::String(asc_get(heap, ptr, depth)?)
            }
            StoreValueKind::Int => Value::Int(i32::from(payload)),
            StoreValueKind::Int8 => Value::Int8(i64::from(payload)),
            StoreValueKind::BigDecimal => {
                let ptr: AscPtr<AscBigDecimal> = AscPtr::from(payload);
                Value::BigDecimal(asc_get(heap, ptr, depth)?)
            }
            StoreValueKind::Bool => Value::Bool(bool::from(payload)),
            StoreValueKind::Array => {
                let ptr: AscEnumArray<StoreValueKind> = AscPtr::from(payload);
                Value::List(asc_get(heap, ptr, depth)?)
            }
            StoreValueKind::Null => Value::Null,
            StoreValueKind::Bytes => {
                let ptr: AscPtr<Uint8Array> = AscPtr::from(payload);
                let array: Vec<u8> = asc_get(heap, ptr, depth)?;
                Value::Bytes(array.as_slice().into())
            }
            StoreValueKind::BigInt => {
                let ptr: AscPtr<AscBigInt> = AscPtr::from(payload);
                let array: Vec<u8> = asc_get(heap, ptr, depth)?;
                Value::BigInt(BigInt::from_signed_bytes_le(&array)?)
            }
        })
    }
}

impl ToAscObj<AscEnum<StoreValueKind>> for Value {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEnum<StoreValueKind>, AscError> {
        let payload = match self {
            Value::String(string) => asc_new(heap, string.as_str())?.into(),
            Value::Int(n) => EnumPayload::from(*n),
            Value::Int8(n) => EnumPayload::from(*n),
            Value::BigDecimal(n) => asc_new(heap, n)?.into(),
            Value::Bool(b) => EnumPayload::from(*b),
            Value::List(array) => asc_new(heap, array.as_slice())?.into(),
            Value::Null => EnumPayload(0),
            Value::Bytes(bytes) => {
                let bytes_obj: AscPtr<Uint8Array> = asc_new(heap, bytes.as_slice())?;
                bytes_obj.into()
            }
            Value::BigInt(big_int) => {
                let bytes_obj: AscPtr<Uint8Array> = asc_new(heap, &*big_int.to_signed_bytes_le())?;
                bytes_obj.into()
            }
        };

        Ok(AscEnum {
            kind: StoreValueKind::get_kind(self),
            _padding: 0,
            payload,
        })
    }
}

impl ToAscObj<AscEnum<JsonValueKind>> for serde_json::Value {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEnum<JsonValueKind>, AscError> {
        use serde_json::Value;

        let payload = match self {
            Value::Null => EnumPayload(0),
            Value::Bool(b) => EnumPayload::from(*b),
            Value::Number(number) => asc_new(heap, &*number.to_string())?.into(),
            Value::String(string) => asc_new(heap, string.as_str())?.into(),
            Value::Array(array) => asc_new(heap, array.as_slice())?.into(),
            Value::Object(object) => asc_new(heap, object)?.into(),
        };

        Ok(AscEnum {
            kind: JsonValueKind::get_kind(self),
            _padding: 0,
            payload,
        })
    }
}

impl ToAscObj<AscJson> for serde_json::Map<String, serde_json::Value> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscJson, AscError> {
        Ok(AscTypedMap {
            entries: asc_new(heap, &*self.iter().collect::<Vec<_>>())?,
        })
    }
}

impl ToAscObj<AscEntity> for Vec<(String, Value)> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscEntity, AscError> {
        Ok(AscTypedMap {
            entries: asc_new(heap, self.as_slice())?,
        })
    }
}

impl ToAscObj<AscEntity> for Vec<(&str, &Value)> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscEntity, AscError> {
        Ok(AscTypedMap {
            entries: asc_new(heap, self.as_slice())?,
        })
    }
}

impl ToAscObj<Array<AscPtr<AscEntity>>> for Vec<Vec<(String, Value)>> {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<Array<AscPtr<AscEntity>>, AscError> {
        let content: Result<Vec<_>, _> = self.iter().map(|x| asc_new(heap, &x)).collect();
        let content = content?;
        Array::new(&content, heap)
    }
}
