use super::Env;
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
    use colored::Colorize;

    let message: String = asc_get(&fenv, msg_ptr, 0)?;

    match log_level {
        0 => {
            let crit_msg = format!("CRITICAL: {message}").red();
            log::log!(target: "wasm-host", log::Level::Warn, "{crit_msg}");

            if env::var("SUBGRAPH_WASM_RUNTIME_TEST").is_ok() {
                // NOTE: if testing, just don't throw anything
                return Ok(());
            }

            return Err(RuntimeError::new(
                "Something bad happened, Terminating runtime!",
            ));
        }
        1 => {
            log::log!(target: "wasm-host", log::Level::Error, "{}", message.truecolor(140, 140, 140))
        }
        2 => {
            log::log!(target: "wasm-host", log::Level::Warn, "{}", message.truecolor(140, 140, 140))
        }
        3 => {
            log::log!(target: "wasm-host", log::Level::Info, "{}", message.truecolor(140, 140, 140))
        }
        4 => {
            log::log!(target: "wasm-host", log::Level::Debug, "{}", message.truecolor(140, 140, 140))
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
