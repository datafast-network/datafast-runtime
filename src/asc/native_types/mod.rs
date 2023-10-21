pub mod array;
pub mod array_buffer;
pub mod r#enum;
pub mod json;
pub mod store;
pub mod string;
pub mod typed_array;
pub mod typed_map;

use typed_array::TypedArray;

pub type Uint8Array = TypedArray<u8>;
