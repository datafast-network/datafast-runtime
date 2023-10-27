#![allow(dead_code)]

use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigInt;
use crate::asc::errors::AscError;
use crate::asc::native_types::string::AscString;
use crate::asc::native_types::AscH160;
use crate::asc::native_types::Uint8Array;
use crate::bignumber::bigint::BigInt;
use crate::host_exports::Env;
use anyhow::Context;
use std::str::FromStr;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;
use wasmer::Type;
use web3::types::H160;

pub const CONVERSION_TYPE: ([Type; 1], [Type; 1]) = ([Type::I32], [Type::I32]);

fn convert_bytes_to_string(bytes: Vec<u8>) -> String {
    let s = String::from_utf8_lossy(&bytes);

    // If the string was re-allocated, that means it was not UTF8.
    if matches!(s, std::borrow::Cow::Owned(_)) {
        log::warn!(
            "Bytes contain invalid UTF8. This may be caused by attempting \
            to convert a value such as an address that cannot be parsed to a unicode string. \
            You may want to use 'toHexString()' instead. String (truncated to 1024 chars): '{}'",
            &s.chars().take(1024).collect::<String>(),
        )
    }

    // The string may have been encoded in a fixed length buffer and padded with null
    // characters, so trim trailing nulls.
    s.trim_end_matches('\u{0000}').to_string()
}

fn convert_string_to_h160(string: &str) -> Result<H160, AscError> {
    // `H160::from_str` takes a hex string with no leading `0x`.
    let s = string.trim_start_matches("0x");
    H160::from_str(s)
        .with_context(|| format!("Failed to convert string to Address/H160: '{}'", s))
        .map_err(|e| AscError::Plain(e.to_string()))
}

pub fn bytes_to_string(
    mut fenv: FunctionEnvMut<Env>,
    bytes_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let bytes: Vec<u8> = asc_get(&fenv, bytes_ptr, 0).unwrap();
    let string = convert_bytes_to_string(bytes);
    let asc_string = asc_new(&mut fenv, &string)?;
    Ok(asc_string)
}

pub fn bytes_to_hex(
    mut fenv: FunctionEnvMut<Env>,
    bytes_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let bytes: Vec<u8> = asc_get(&fenv, bytes_ptr, 0).unwrap();
    let asc_hex = asc_new(&mut fenv, &format!("0x{}", hex::encode(bytes)))?;
    Ok(asc_hex)
}

pub fn big_int_to_string(
    mut fenv: FunctionEnvMut<Env>,
    big_int_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let big_int: BigInt = asc_get(&fenv, big_int_ptr, 0).unwrap();
    let big_int_string = asc_new(&mut fenv, &big_int.to_string())?;
    Ok(big_int_string)
}

pub fn big_int_to_hex(
    mut fenv: FunctionEnvMut<Env>,
    big_int_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let big_int: BigInt = asc_get(&fenv, big_int_ptr, 0).unwrap();
    if big_int == 0.into() {
        let result = asc_new(&mut fenv, "0x0")?;
        Ok(result)
    } else {
        let bytes = big_int.to_bytes_be().1;
        let result = asc_new(
            &mut fenv,
            &format!("0x{}", hex::encode(bytes).trim_start_matches('0')),
        )?;
        Ok(result)
    }
}

pub fn string_to_h160(
    mut fenv: FunctionEnvMut<Env>,
    string_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscH160>, RuntimeError> {
    let string: String = asc_get(&fenv, string_ptr, 0).unwrap();
    let h160 = convert_string_to_h160(&string)?;
    let result = asc_new(&mut fenv, &h160)?;
    Ok(result)
}

//bytes_to_base58
pub fn bytes_to_base58(
    mut fenv: FunctionEnvMut<Env>,
    bytes_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let bytes: Vec<u8> = asc_get(&fenv, bytes_ptr, 0).unwrap();
    let result = asc_new(&mut fenv, &bs58::encode(bytes).into_string())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use super::*;
    use crate::host_fn_test;

    host_fn_test!(test_bytes_to_hex, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "0x616263");
    });

    host_fn_test!(test_bytes_to_string, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "abc");
    });

    host_fn_test!(test_hex_to_bytes, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "abc");
    });

    host_fn_test!(test_big_int_to_string, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "1000");
    });

    host_fn_test!(test_big_int_to_hex, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "0x3e8"); //0x3E8
    });

    host_fn_test!(test_bytes_to_base58, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let string_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(string_result, "ZiCa");
    });
}
