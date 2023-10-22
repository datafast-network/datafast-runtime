use crate::asc::base::AscIndexId;
use crate::asc::base::AscType;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::base::{AscHeap, FromAscObj};
use crate::asc::errors::AscError;

use std::mem::size_of_val;

pub struct AscString {
    // Not included in memory layout
    // In number of UTF-16 code units (2 bytes each).
    byte_length: u32,
    // #data
    // The sequence of UTF-16LE code units that form the string.
    pub content: Box<[u16]>,
}

impl AscIndexId for AscString {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::String;
}

impl AscString {
    pub fn new(content: &[u16]) -> Result<Self, AscError> {
        if size_of_val(content) > u32::MAX as usize {
            return Err(AscError::Plain(
                "string cannot fit in WASM memory".to_string(),
            ));
        }

        Ok(AscString {
            byte_length: content.len() as u32,
            content: content.into(),
        })
    }

    pub fn content(&self) -> &[u16] {
        &self.content
    }
}

impl AscType for AscString {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let mut content: Vec<u8> = Vec::new();

        // Write the code points, in little-endian (LE) order.
        for &code_unit in self.content.iter() {
            let low_byte = code_unit as u8;
            let high_byte = (code_unit >> 8) as u8;
            content.push(low_byte);
            content.push(high_byte);
        }

        let header_size = 20;
        let total_size = (self.byte_length as usize * 2) + header_size;
        let total_capacity = total_size.next_power_of_two();
        let extra_capacity = total_capacity - total_size;
        content.extend(std::iter::repeat(0).take(extra_capacity));

        Ok(content)
    }

    /// The Rust representation of an Asc object as layed out in Asc memory.
    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        // UTF-16 (used in assemblyscript) always uses one
        // pair of bytes per code unit.
        // https://mathiasbynens.be/notes/javascript-encoding
        // UTF-16 (16-bit Unicode Transformation Format) is an
        // extension of UCS-2 that allows representing code points
        // outside the BMP. It produces a variable-length result
        // of either one or two 16-bit code units per code point.
        // This way, it can encode code points in the range from 0
        // to 0x10FFFF.

        let mut content = Vec::new();
        for pair in asc_obj.chunks(2) {
            let code_point_bytes = [
                pair[0],
                *pair.get(1).ok_or_else(|| {
                    AscError::Plain(
                        "Attempted to read past end of string content bytes chunk".to_string(),
                    )
                })?,
            ];
            let code_point = u16::from_le_bytes(code_point_bytes);
            content.push(code_point);
        }
        AscString::new(&content)
    }

    fn content_len(&self, _asc_bytes: &[u8]) -> usize {
        self.byte_length as usize * 2 // without extra_capacity, and times 2 because the content is measured in u8s
    }
}

impl ToAscObj<AscString> for str {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
        Ok(AscString::new(&self.encode_utf16().collect::<Vec<_>>())?)
    }
}

// impl ToAscObj<AscString> for &str {
//     fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscString, AscError> {
//         Ok(AscString::new(&self.encode_utf16().collect::<Vec<_>>())?)
//     }
// }

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
