/// Colors:
/// https://gist.github.com/raghav4/48716264a0f426cf95e4342c21ada8e7
#[macro_export]
macro_rules! log_info {
    ($target:ident, $msg:expr) => {
        log::info!(target: &format!("\x1b[32m{}",stringify!($target)), "{}\x1B[0m", $msg);
    };
    ($target:ident, $msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}{}", $msg, keys_message.replace("Object", ""));
        log::info!(target: &format!("\x1b[32m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
    ($target:ident; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}",keys_message.replace("Object", ""));
        log::info!(target: &format!("\x1b[32m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:ident, $msg:expr) => {
        log::error!(target: &format!("\x1b[31m{}",stringify!($target)), "{}\x1B[0m", $msg);
    };
    ($target:ident,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}{}", $msg, keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
    ($target:ident; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}",keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:ident, $msg:expr) => {
        log::warn!(target: &format!("\x1b[33m{}",stringify!($target)), "{}\x1B[0m", $msg);
    };
    ($target:ident,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}{}", $msg, keys_message.replace("Object", ""));
        log::warn!(target: &format!("\x1b[33m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
    ($target:ident; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}",keys_message.replace("Object", ""));
        log::warn!(target: &format!("\x1b[33m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:ident, $msg:expr) => {
        log::debug!(target: &format!("\x1b[90m{}", stringify!($target)), "{}\x1B[0m", $msg);
    };
    ($target:ident,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}{}", $msg, keys_message.replace("Object", ""));
        log::debug!(target: &format!("\x1b[90m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
    ($target:ident; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("{}",keys_message.replace("Object", ""));
        log::debug!(target: &format!("\x1b[90m{}\x1B[0m",stringify!($target)), "{}", result_message);
    };
}

#[macro_export]
macro_rules! log_critical {
    ($target:ident, $msg:expr) => {
        log::error!(target: &format!("\x1b[31m{}",stringify!($target)), "\x1b[31m[CRITICAL]!\x1B[0m {}", $msg);
    };
    ($target:ident,$msg:expr; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[31m!!!CRITICAL!!!\x1B[0m {}{}", $msg, keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",stringify!($target)), "{}", result_message);
    };
    ($target:ident; $($key:expr => $value:expr),*) => {
        let keys_message = format!("{:?}", serde_json::json!({$($key: $value),*}));
        let result_message = format!("\x1b[31m!!!CRITICAL!!!\x1B[0m{}",keys_message.replace("Object", ""));
        log::error!(target: &format!("\x1b[31m{}",stringify!($target)), "{}", result_message);
    };
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_loggers_macros() {
        env_logger::try_init().unwrap_or_default();
        log_info!(test_loggers_macros, "message only");
        log_info!(test_loggers_macros, "KeyValue"; 123 => "value1", "key2" => "value2");
        log_info!(test_loggers_macros; "key1" => "value1", "key2" => "value2");
        log_warn!(test_loggers_macros, "message only");
        log_warn!(test_loggers_macros, "KeyValue"; "key1" => "value1", "key2" => "value2");
        log_warn!(test_loggers_macros; "key1" => "value1", "key2" => "value2");
        log_error!(test_loggers_macros, "message only");
        log_error!(test_loggers_macros, "KeyValue"; "key1" => "value1", "key2" => "value2");
        log_error!(test_loggers_macros; "key1" => "value1", "key2" => "value2");
        log_critical!(test_loggers_macros, "KeyValue"; "key1" => "value1", "key2" => "value2");
        log::info!("Default");
        log::warn!("Default");
        log::error!("Default");
    }
}
