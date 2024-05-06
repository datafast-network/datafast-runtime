use super::Env;
use crate::critical;
use crate::debug;
use crate::error;
use crate::info;
use crate::warn;
use df_types::asc::base::asc_get;
use df_types::asc::base::AscPtr;
use df_types::asc::native_types::string::AscString;
use df_types::wasmer::FunctionEnvMut;
use df_types::wasmer::RuntimeError;
use std::env;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let datasource_name = fenv.data().host_name.clone().to_string();
    let message: String = asc_get(&fenv, msg_ptr, 0)?;
    match log_level {
        0 => {
            critical!(WasmHost, message; datasource => datasource_name);
            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => {
            error!(WasmHost, message; datasource => datasource_name);
        }
        2 => {
            warn!(WasmHost, message; datasource => datasource_name);
        }
        3 => {
            info!(WasmHost, message; datasource => datasource_name);
        }
        4 => {
            debug!(WasmHost, message; datasource => datasource_name);
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
