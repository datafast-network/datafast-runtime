use super::Env;
use crate::critical;
use crate::debug;
use crate::error;
use crate::info;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::string::AscString;
use crate::warn;
use std::env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let datasource_name = fenv.data().datasource_name.clone().to_string();
    let message: String = asc_get(&fenv, msg_ptr, 0)?;
    match log_level {
        0 => {
            critical!(wasm_host, message; datasource => datasource_name);
            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => {
            error!(wasm_host, message; datasource => datasource_name);
        }
        2 => {
            warn!(wasm_host, message; datasource => datasource_name);
        }
        3 => {
            info!(wasm_host, message; datasource => datasource_name);
        }
        4 => {
            debug!(wasm_host, message; datasource => datasource_name);
        }
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
