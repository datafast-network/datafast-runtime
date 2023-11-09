/// Colors:
/// https://gist.github.com/raghav4/48716264a0f426cf95e4342c21ada8e7
#[macro_export]
macro_rules! log_info {
    ($target:expr, $msg:expr) => {
        log::info!(target: &format!("\x1b[32m{}",$target), "\x1b[32m{}\x1B[0m", $msg);
    };
    ($target:expr,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[32m{}\x1B[0m \x1b[36m{}\x1B[0m", $msg, keys_message.replace("Object", ""));
        log::info!(target: &format!("\x1b[32m{}",$target), "{}", result_message);
    };
    ($target:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[36m{}\x1B[0m",keys_message.replace("Object", ""));
        log::info!(target: &format!("\x1b[32m{}",$target), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $msg:expr) => {
        log::error!(target: &format!("\x1b[31m{}",$target), "\x1b[31m{}\x1B[0m", $msg);
    };
    ($target:expr,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[31m{}\x1B[0m \x1b[36m{}\x1B[0m", $msg, keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",$target), "{}", result_message);
    };
    ($target:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[31m{}\x1B[0m",keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",$target), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $msg:expr) => {
        log::warn!(target: &format!("\x1b[33m{}",$target), "\x1b[33m{}\x1B[0m", $msg);
    };
    ($target:expr,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[33m{}\x1B[0m \x1b[36m{}\x1B[0m", $msg, keys_message.replace("Object", ""));
        log::warn!(target: &format!("\x1b[33m{}",$target), "{}", result_message);
    };
    ($target:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[33m{}\x1B[0m",keys_message.replace("Object", ""));
        log::warn!(target: &format!("\x1b[33m{}",$target), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $msg:expr) => {
        log::debug!(target: &format!("\x1b[90m{}", $target), "\x1b[90m{}\x1B[0m", $msg);
    };
    ($target:expr,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[90m{}\x1B[0m \x1b[90m{}\x1B[0m", $msg, keys_message.replace("Object", ""));
        log::debug!(target: &format!("\x1b[90m{}",$target), "{}", result_message);
    };
    ($target:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[90m{}\x1B[0m",keys_message.replace("Object", ""));
        log::debug!(target: &format!("\x1b[90m{}",$target), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_critical {
    ($target:expr, $msg:expr) => {
        log::error!(target: &format!("\x1b[31m{}",$target), "\x1b[31m[CRITICAL]! {}\x1B[0m", $msg);
    };
    ($target:expr,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[31m!!!CRITICAL!!! {}\x1B[0m \x1b[36m{}\x1B[0m", $msg, keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",$target), "{}", result_message);
    };
    ($target:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("!!!CRITICAL!!!\x1b[31m{}\x1B[0m",keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",$target), "{}", result_message);
    };
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_loggers_macros() {
        env_logger::try_init().unwrap_or_default();
        log_info!("Test", "22222"; "key1" => "value1", "key2" => "value2");
        log_warn!("Test", "22222"; "key1" => "value1", "key2" => "value2");
        log_error!("Test", "22222"; "key1" => "value1", "key2" => "value2");
        log_critical!("Test", "22222"; "key1" => "value1", "key2" => "value2");
    }
}
