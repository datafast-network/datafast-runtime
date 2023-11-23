macro_rules! from_vec_json_value {
    ($struct_name:ident; $($field_name:ident => $field_type:ty),*) => {
        impl TryFrom<Vec<serde_json::Value>> for $struct_name {
            type Error = SourceError;

            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            #[allow(unused_variables)]
            fn try_from(values: Vec<serde_json::Value>) -> Result<Self, Self::Error> {
                let mut this = Self::default();
                let mut idx = 0;

                $(
                    let value = values.get(idx).cloned().unwrap();
                    this.$field_name = serde_json::from_value(value).map_err(|_| SourceError::TrinoSerializeFail)?;
                    idx += 1;
                )*

                    Ok(this)
            }
        }
    };
}

pub(crate) use from_vec_json_value;
