macro_rules! impl_asc_type_struct {
    ($struct_name:ident; $($field_name:ident => $field_type:ty),*) => {
        impl crate::asc::asc_base::AscType for $struct_name {
            fn to_asc_bytes(&self) -> Result<Vec<u8>, crate::asc::errors::AscError> {
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
            fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, crate::asc::errors::AscError> {
                // Sanity check
                let content_size = std::mem::size_of::<Self>();
                let aligned_size = crate::asc::asc_base::padding_to_16(content_size);

                if crate::asc::asc_base::HEADER_SIZE + asc_obj.len() == aligned_size + content_size {
                    return Err(crate::asc::errors::AscError::SizeNotMatch);
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
                        crate::asc::errors::AscError::Plain("Attempted to read past end of array".to_string())
                    })?;
                    let $field_name = crate::asc::asc_base::AscType::from_asc_bytes(&field_data)?;
                    offset += field_size;
                )*

                Ok(Self {
                    $($field_name,)*
                })
            }
        }
    };
}

pub(crate) use impl_asc_type_struct;
