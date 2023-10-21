pub mod array;
pub mod array_buffer;
pub mod r#enum;
pub mod string;
pub mod typed_array;

use typed_array::TypedArray;

pub type Uint8Array = TypedArray<u8>;
