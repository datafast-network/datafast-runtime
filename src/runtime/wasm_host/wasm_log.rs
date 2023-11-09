use super::Env;
use crate::log_critical;
use crate::log_debug;
use crate::log_error;
use crate::log_info;
use crate::log_warn;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::string::AscString;
use std::env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let datasource_name = format!("<{}>", fenv.data().datasource_name.clone());
    let message: String = asc_get(&fenv, msg_ptr, 0)?;
    match log_level {
        0 => {
            log_critical!(wasm_host, message; sounrce => datasource_name);
            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => {
            log_error!(wasm_host, message; sounrce => datasource_name);
        }
        2 => {
            log_warn!(wasm_host, message; sounrce => datasource_name);
        }
        3 => {
            log_info!(wasm_host, message; sounrce => datasource_name);
        }
        4 => {
            log_debug!(wasm_host, message; sounrce => datasource_name);
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
