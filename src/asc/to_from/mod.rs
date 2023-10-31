use super::base::asc_get;
use super::base::asc_new;
use super::base::AscHeap;
use super::base::AscIndexId;
use super::base::AscPtr;
use super::base::AscType;
use super::base::AscValue;
use super::base::FromAscObj;
use super::base::ToAscObj;
use super::errors::AscError;
use super::native_types::array::Array;
use super::native_types::string::AscString;
use super::native_types::typed_array::TypedArray;
use super::native_types::typed_map::AscTypedMap;
use super::native_types::typed_map::AscTypedMapEntry;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FromIterator;

///! Implementations of `ToAscObj` and `FromAscObj` for Rust types.
///! Standard Rust types go in `protobuf` and external types in `external.rs`.

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
