use df_types::errors::AscError;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::AscType;
use semver::Version;
use std::mem::size_of;
use std::mem::size_of_val;

pub struct AscString {
    // In number of UTF-16 code units (2 bytes each).
    length: u32,
    // The sequence of UTF-16LE code units that form the string.
    pub content: Box<[u16]>,
}

impl AscString {
    pub fn new(content: &[u16]) -> Result<Self, AscError> {
        if size_of_val(content) > u32::max_value() as usize {
            return Err(AscError::SizeNotFit);
        }

        Ok(AscString {
            length: content.len() as u32,
            content: content.into(),
        })
    }
}

impl AscType for AscString {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let mut asc_layout: Vec<u8> = Vec::new();

        let length: [u8; 4] = self.length.to_le_bytes();
        asc_layout.extend(length);

        // Write the code points, in little-endian (LE) order.
        for &code_unit in self.content.iter() {
            let low_byte = code_unit as u8;
            let high_byte = (code_unit >> 8) as u8;
            asc_layout.push(low_byte);
            asc_layout.push(high_byte);
        }

        Ok(asc_layout)
    }

    /// The Rust representation of an Asc object as layed out in Asc memory.
    fn from_asc_bytes(asc_obj: &[u8], _api_version: &Version) -> Result<Self, AscError> {
        // Pointer for our current position within `asc_obj`,
        // initially at the start of the content skipping `length`.
        let mut offset = size_of::<i32>();

        let length = asc_obj.get(..offset).ok_or(AscError::Plain(
            "String bytes not long enough to contain length".to_string(),
        ))?;

        // Does not panic - already validated slice length == size_of::<i32>.
        let length = i32::from_le_bytes(length.try_into().unwrap());
        if length.checked_mul(2).and_then(|l| l.checked_add(4)) != asc_obj.len().try_into().ok() {
            return Err(AscError::Plain(
                "String length header does not equal byte length".to_string(),
            ));
        }

        // Prevents panic when accessing offset + 1 in the loop
        if asc_obj.len() % 2 != 0 {
            return Err(AscError::Plain("Invalid string length".to_string()));
        }

        // UTF-16 (used in assemblyscript) always uses one
        // pair of bytes per code unit.
        // https://mathiasbynens.be/notes/javascript-encoding
        // UTF-16 (16-bit Unicode Transformation Format) is an
        // extension of UCS-2 that allows representing code points
        // outside the BMP. It produces a variable-length result
        // of either one or two 16-bit code units per code point.
        // This way, it can encode code points in the range from 0
        // to 0x10FFFF.

        // Read the content.
        let mut content = Vec::new();
        while offset < asc_obj.len() {
            let code_point_bytes = [asc_obj[offset], asc_obj[offset + 1]];
            let code_point = u16::from_le_bytes(code_point_bytes);
            content.push(code_point);
            offset += size_of::<u16>();
        }
        AscString::new(&content)
    }

    fn asc_size<H: AscHeap + ?Sized>(ptr: AscPtr<Self>, heap: &H) -> Result<u32, AscError> {
        let length = ptr.read_u32(heap)?;
        let length_size = size_of::<u32>() as u32;
        let code_point_size = size_of::<u16>() as u32;
        let data_size = code_point_size.checked_mul(length);
        let total_size = data_size.and_then(|d| d.checked_add(length_size));
        total_size
            .ok_or_else(|| AscError::Plain("Overflowed when getting size of string".to_string()))
    }
}
