use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::native_types::string::AscString;
use std::env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let string: String = asc_get(&fenv, msg_ptr, 0)?;

    match log_level {
        0 => {
            log::log!(target: "subgraph-wasm-host", log::Level::max(), "CRITICAL: {string}");

            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => log::log!(target: "subgraph-wasm-host", log::Level::Error, "{string}"),
        2 => log::log!(target: "subgraph-wasm-host", log::Level::Warn, "{string}"),
        3 => log::log!(target: "subgraph-wasm-host", log::Level::Info, "{string}"),
        4 => log::log!(target: "subgraph-wasm-host", log::Level::Debug, "{string}"),
        _ => return Err(RuntimeError::new("Invalid log level!!")),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::super::test::*;
    use crate::host_fn_test;

    host_fn_test!("TestTypes", test_log, host {});
}
