use super::base::AscHeap;
use super::base::AscValue;
use super::base::FromAscObj;
use super::base::ToAscObj;
use super::errors::AscError;
use super::native_types::typed_array::TypedArray;

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
