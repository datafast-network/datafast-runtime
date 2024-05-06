use super::Env;
use df_types::asc::base::asc_get;
use df_types::asc::base::AscPtr;
use df_types::asc::native_types::string::AscString;
use df_types::wasmer::FunctionEnvMut;
use df_types::wasmer::RuntimeError;

pub fn abort(
    fenv: FunctionEnvMut<Env>,
    message_ptr: AscPtr<AscString>,
    file_name_ptr: AscPtr<AscString>,
    line_number: u32,
    column_number: u32,
) -> Result<(), RuntimeError> {
    let message: Option<String> = match message_ptr.is_null() {
        false => Some(asc_get(&fenv, message_ptr, 0)?),
        true => None,
    };
    let file_name: Option<String> = match file_name_ptr.is_null() {
        false => Some(asc_get(&fenv, file_name_ptr, 0)?),
        true => None,
    };
    let line_number = match line_number {
        0 => None,
        _ => Some(line_number),
    };
    let column_number = match column_number {
        0 => None,
        _ => Some(column_number),
    };

    let message = message
        .map(|message| format!("message: {}", message))
        .unwrap_or_else(|| "no message".into());

    let location = match (file_name, line_number, column_number) {
        (None, None, None) => "an unknown location".into(),
        (Some(file_name), None, None) => file_name,
        (Some(file_name), Some(line_number), None) => {
            format!("{}, line {}", file_name, line_number)
        }
        (Some(file_name), Some(line_number), Some(column_number)) => format!(
            "{}, line {}, column {}",
            file_name, line_number, column_number
        ),
        _ => unreachable!(),
    };

    Err(RuntimeError::new(format!(
        "Mapping aborted at {}, with {}",
        location, message
    )))
}

#[cfg(test)]
mod test {
    use super::super::test::*;
    use crate::host_fn_test;

    host_fn_test!("TestGlobalVar", test_global_var, host {});
}
