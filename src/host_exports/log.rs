use std::env;

use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::native_types::string::AscString;
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
            eprintln!("CRITICAL!!!!!!: {string}");

            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => log::error!("{string}"),
        2 => log::warn!("{string}"),
        3 => log::info!("{string}"),
        4 => log::debug!("{string}"),
        _ => return Err(RuntimeError::new("Invalid log level!!")),
    }

    Ok(())
}

pub fn log_dkm(fenv: FunctionEnvMut<Env>, name: AscPtr<AscString>) -> Result<(), RuntimeError> {
    let string: String = asc_get(&fenv, name, 0)?;
    log::error!("DKM {string}");
    Ok(())
}
#[cfg(test)]
mod test {
    use super::super::test::*;
    use crate::host_fn_test;

    host_fn_test!(test_log, host {});

    host_fn_test!(test_dkm, host {});
}
