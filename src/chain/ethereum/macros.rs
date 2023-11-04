#[macro_export]
macro_rules! impl_uint8_array_for_web3_type {
    ($type:ty, $size: expr) => {
        impl ToAscObj<Uint8Array> for $type {
            fn to_asc_obj<H: AscHeap + ?Sized>(
                &self,
                heap: &mut H,
            ) -> Result<Uint8Array, AscError> {
                self.0.to_asc_obj(heap)
            }
        }
        impl FromAscObj<Uint8Array> for $type {
            fn from_asc_obj<H: AscHeap + ?Sized>(
                typed_array: Uint8Array,
                heap: &H,
                depth: usize,
            ) -> Result<Self, AscError> {
                let data = <[u8; $size]>::from_asc_obj(typed_array, heap, depth)?;
                Ok(Self(data))
            }
        }
    };
}
#[macro_export]
macro_rules! impl_from_big_int_to_web3_type {
    ($type:ty, $size:expr) => {
        impl ToAscObj<AscBigInt> for $type {
            fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscBigInt, AscError> {
                let mut bytes: [u8; $size] = [0; $size];
                self.to_little_endian(&mut bytes);
                bytes.to_asc_obj(heap)
            }
        }
        impl FromAscObj<AscBigInt> for $type {
            fn from_asc_obj<H: AscHeap + ?Sized>(
                obj: AscBigInt,
                heap: &H,
                depth: usize,
            ) -> Result<Self, AscError> {
                let bytes = Vec::from_asc_obj(obj, heap, depth)?;
                let hex_str = hex::encode(bytes.clone());
                let big = BigInt::from_hex(hex_str).unwrap();
                Ok(big.into())
            }
        }
    };
}
