#[macro_export]
macro_rules! impl_asc_type {
    ($($T:ty),*) => {
        $(
            impl AscType for $T {
                fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
                    Ok(self.to_le_bytes().to_vec())
                }

                fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, AscError> {
                    let bytes = asc_obj.try_into().map_err(|_| {
                        AscError::Plain(format!("Incorrect size for {}. Expected {}, got {},", stringify!($T),
                            std::mem::size_of::<Self>(),
                            asc_obj.len()))
                    })?;

                    Ok(Self::from_le_bytes(bytes))
                }
            }

            impl AscValue for $T {}
        )*
    };
}

#[macro_export]
macro_rules! impl_asc_type_struct {
    ($struct_name:ident; $($field_name:ident => $field_type:ty),*) => {
        impl crate::runtime::asc::base::AscType for $struct_name  {
            fn to_asc_bytes(&self) -> Result<Vec<u8>, crate::errors::AscError> {
                let in_memory_byte_count = std::mem::size_of::<Self>();
                let mut bytes = Vec::with_capacity(in_memory_byte_count);

                let mut offset = 0;
                // max field alignment will also be struct alignment which we need to pad the end
                let mut max_align = 0;

                $(
                    let field_align = std::mem::align_of::<$field_type>();
                    let misalignment = offset % field_align;

                    if misalignment > 0 {
                        let padding_size = field_align - misalignment;

                        bytes.extend_from_slice(&vec![0; padding_size]);

                        offset += padding_size;
                    }

                    let field_bytes = self.$field_name.to_asc_bytes()?;

                    bytes.extend_from_slice(&field_bytes);

                    offset += field_bytes.len();

                    if max_align < field_align {
                        max_align = field_align;
                    }
                )*

                // pad end of struct data if needed
                let struct_misalignment = offset % max_align;

                if struct_misalignment > 0 {
                    let padding_size = max_align - struct_misalignment;

                    bytes.extend_from_slice(&vec![0; padding_size]);
                }

                // **Important** AssemblyScript and `repr(C)` in Rust does not follow exactly
                // the same rules always. One caveats is that some struct are packed in AssemblyScript
                // but padded for alignment in `repr(C)` like a struct `{ one: AscPtr, two: AscPtr, three: AscPtr, four: u64 }`,
                // it appears this struct is always padded in `repr(C)` by Rust whatever order is tried.
                // However, it's possible to packed completely this struct in AssemblyScript and avoid
                // any padding.
                //
                // To overcome those cases where re-ordering never work, you will need to add an explicit
                // _padding field to account for missing padding and pass this check.
                assert_eq!(bytes.len(), in_memory_byte_count, "Alignment mismatch for {}, re-order fields or explicitely add a _padding field", stringify!(#struct_name));
                Ok(bytes)
            }

            #[allow(unused_variables)]
            #[allow(unused_assignments)]
            fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, crate::errors::AscError> {
                // Sanity check
                let content_size = std::mem::size_of::<Self>();
                let aligned_size = crate::runtime::asc::base::padding_to_16(content_size);

                if crate::runtime::asc::base::HEADER_SIZE + asc_obj.len() == aligned_size + content_size {
                    return Err(crate::errors::AscError::SizeNotMatch);
                }

                let mut offset = 0;

                $(
                    // skip padding
                    let field_align = std::mem::align_of::<$field_type>();
                    let misalignment = offset % field_align;
                    if misalignment > 0 {
                        let padding_size = field_align - misalignment;

                        offset += padding_size;
                    }

                    let field_size = std::mem::size_of::<$field_type>();
                    let field_data = asc_obj.get(offset..(offset + field_size)).ok_or_else(|| {
                        crate::errors::AscError::Plain("Attempted to read past end of array".to_string())
                    })?;
                    let $field_name = crate::runtime::asc::base::AscType::from_asc_bytes(&field_data, api_version)?;
                    offset += field_size;
                )*

                Ok(Self {
                    $($field_name,)*
                })
            }
        }
    };
    ($struct_name:ident $(< $( $generic_name:tt $( : $generic_type:tt $(+ $generic_type_n:tt )* )? ),+ >)?; $($field_name:ident => $field_type:ty),*) => {
        impl $(< $( $generic_name $( : $generic_type $(+ $generic_type_n )* )? ),+ >)? crate::runtime::asc::base::AscType for $struct_name  $(< $( $generic_name ),+ >)? {
            fn to_asc_bytes(&self) -> Result<Vec<u8>, crate::errors::AscError> {
                let in_memory_byte_count = std::mem::size_of::<Self>();
                let mut bytes = Vec::with_capacity(in_memory_byte_count);

                let mut offset = 0;
                // max field alignment will also be struct alignment which we need to pad the end
                let mut max_align = 0;

                $(
                    let field_align = std::mem::align_of::<$field_type>();
                    let misalignment = offset % field_align;

                    if misalignment > 0 {
                        let padding_size = field_align - misalignment;

                        bytes.extend_from_slice(&vec![0; padding_size]);

                        offset += padding_size;
                    }

                    let field_bytes = self.$field_name.to_asc_bytes()?;

                    bytes.extend_from_slice(&field_bytes);

                    offset += field_bytes.len();

                    if max_align < field_align {
                        max_align = field_align;
                    }
                )*

                // pad end of struct data if needed
                let struct_misalignment = offset % max_align;

                if struct_misalignment > 0 {
                    let padding_size = max_align - struct_misalignment;

                    bytes.extend_from_slice(&vec![0; padding_size]);
                }

                // **Important** AssemblyScript and `repr(C)` in Rust does not follow exactly
                // the same rules always. One caveats is that some struct are packed in AssemblyScript
                // but padded for alignment in `repr(C)` like a struct `{ one: AscPtr, two: AscPtr, three: AscPtr, four: u64 }`,
                // it appears this struct is always padded in `repr(C)` by Rust whatever order is tried.
                // However, it's possible to packed completely this struct in AssemblyScript and avoid
                // any padding.
                //
                // To overcome those cases where re-ordering never work, you will need to add an explicit
                // _padding field to account for missing padding and pass this check.
                assert_eq!(bytes.len(), in_memory_byte_count, "Alignment mismatch for {}, re-order fields or explicitely add a _padding field", stringify!(#struct_name));
                Ok(bytes)
            }

            #[allow(unused_variables)]
            #[allow(unused_assignments)]
            fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, crate::errors::AscError> {
                // Sanity check
                let content_size = std::mem::size_of::<Self>();
                let aligned_size = crate::runtime::asc::base::padding_to_16(content_size);

                if crate::runtime::asc::base::HEADER_SIZE + asc_obj.len() == aligned_size + content_size {
                    return Err(crate::errors::AscError::SizeNotMatch);
                }

                let mut offset = 0;

                $(
                    // skip padding
                    let field_align = std::mem::align_of::<$field_type>();
                    let misalignment = offset % field_align;
                    if misalignment > 0 {
                        let padding_size = field_align - misalignment;

                        offset += padding_size;
                    }

                    let field_size = std::mem::size_of::<$field_type>();
                    let field_data = asc_obj.get(offset..(offset + field_size)).ok_or_else(|| {
                        crate::errors::AscError::Plain("Attempted to read past end of array".to_string())
                    })?;
                    let $field_name = crate::runtime::asc::base::AscType::from_asc_bytes(&field_data, api_version)?;
                    offset += field_size;
                )*

                Ok(Self {
                    $($field_name,)*
                })
            }
        }
    }
}

#[macro_export]
macro_rules! impl_asc_type_enum {
    ($enum_name:ident; $($variant_name:ident => $variant_index:tt),*) => {
        impl crate::runtime::asc::base::AscType for $enum_name  {
            fn to_asc_bytes(&self) -> Result<Vec<u8>, crate::errors::AscError> {
               let discriminant: u32 = match self {
                    $($enum_name::$variant_name => $variant_index,)*
                };
                discriminant.to_asc_bytes()
            }

            fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, crate::errors::AscError> {
                let u32_bytes = ::std::convert::TryFrom::try_from(asc_obj)
                    .map_err(|_| crate::errors::AscError::Plain("invalid Kind".to_string()))?;
                let discriminant = u32::from_le_bytes(u32_bytes);
                match discriminant {
                    $($variant_index => Ok($enum_name::$variant_name),)*
                    _ => Err(crate::errors::AscError::Plain("invalid Kind".to_string()))
                }
            }
        }
    };
    //enum with tuple
    ($enum_name:ident $(< $( $generic_name:tt $( : $generic_type:tt $(+ $generic_type_n:tt )* )? ),+ >)?; $($variant_name:ident($variant_type:tt) => $variant_index:tt),*) => {
        impl $(< $( $generic_name $( : $generic_type $(+ $generic_type_n )* )? ),+ >)? crate::runtime::asc::base::AscType
        for $enum_name  $(< $( $generic_name ),+ >)? where $($variant_type: crate::runtime::asc::base::AscType),+
        {
            fn to_asc_bytes(&self) -> Result<Vec<u8>, crate::errors::AscError> {
                match self {
                    $($enum_name::$variant_name(value) => value.to_asc_bytes(),)*
                }
            }

            fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, crate::errors::AscError> {
                let u32_bytes = ::std::convert::TryFrom::try_from(asc_obj)
                    .map_err(|_| crate::errors::AscError::Plain("invalid enum type".to_string()))?;
                let discriminant = u32::from_le_bytes(u32_bytes);
                match discriminant {
                    $($variant_index => Ok($enum_name::$variant_name($variant_type::from_asc_bytes(asc_obj, api_version)?)),)*
                    _ => Err(crate::errors::AscError::Plain("invalid enum type".to_string()))
                }
            }
        }
    }
}
