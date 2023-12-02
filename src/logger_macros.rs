/// Colors:
/// https://gist.github.com/raghav4/48716264a0f426cf95e4342c21ada8e7
#[macro_export]
macro_rules! generate_log_message {
    ($log_level:ident, $target:ident, $msg: expr) => {
        let msg = match stringify!($log_level) {
            "warn" => format!("\x1b[33m{}\x1b[0m", $msg),
            "error" => format!("\x1b[31m{}\x1b[0m", $msg),
            "debug" => format!("\x1b[34m{}\x1b[0m", $msg),
            _ => format!("{}", $msg),
        };

        log::$log_level!(target: &format!("{}",stringify!($target)), "{}", msg);
    };
    ($log_level:ident, $target:ident, $msg:expr; $($key:ident => $value:expr),*) => {
        let msg = match stringify!($log_level) {
            "warn" => format!("\x1b[33m{}\x1b[0m", $msg),
            "error" => format!("\x1b[31m{}\x1b[0m", $msg),
            "debug" => format!("\x1b[34m{}\x1b[0m", $msg),
            _ => format!("{}", $msg),
        };

        let keys_message = vec![
            $(
                format!("\x1b[2m{}\x1b[0m\x1b[32;2m=\x1b[0m\x1b[2m{}\x1b[0m", stringify!($key), $value),
            )*
        ].join(", ");
        let result_message = format!("{}\n {}", msg, keys_message);
        log::$log_level!(target: &format!("{}",stringify!($target)), "{}", result_message);
    };
}

#[macro_export]
macro_rules! info {
    ($target:ident, $msg:expr) => {
        $crate::generate_log_message!(info, $target, $msg);
    };
    ($target:ident, $msg:expr; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(info, $target, $msg; $($key => $value),*);
    };
    ($target:ident; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(info, $target, ""; $($key => $value),*);
    };
}

#[macro_export]
macro_rules! error {
    ($target:ident, $msg:expr) => {
        $crate::generate_log_message!(error, $target, $msg);
    };
    ($target:ident,$msg:expr; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(error, $target, $msg; $($key => $value),*);
    };
    ($target:ident; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(error, $target, ""; $($key => $value),*);
    };
}

#[macro_export]
macro_rules! warn {
    ($target:ident, $msg:expr) => {
        $crate::generate_log_message!(warn, $target, $msg);
    };
    ($target:ident,$msg:expr; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(warn, $target, $msg; $($key => $value),*);
    };
    ($target:ident; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(warn, $target, ""; $($key => $value),*);
    };
}

#[macro_export]
macro_rules! debug {
    ($target:ident, $msg:expr) => {
        $crate::generate_log_message!(debug, $target, $msg);
    };
    ($target:ident, $msg:expr; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(debug, $target, $msg; $($key => $value),*);
    };
    ($target:ident; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(debug, $target, ""; $($key => $value),*);
    };
}

#[macro_export]
macro_rules! critical {
    ($target:ident, $msg:expr) => {
        let msg = format!("!!![CRITICAL]!!! {}", $msg);
        $crate::generate_log_message!(error, $target, msg);
    };
    ($target:ident,$msg:expr; $($key:ident => $value:expr),*) => {
        let msg = format!("!!![CRITICAL]!!! {}", $msg);
        $crate::generate_log_message!(error, $target, msg; $($key => $value),*);
    };
    ($target:ident; $($key:ident => $value:expr),*) => {
        $crate::generate_log_message!(error, $target, ""; $($key => $value),*);
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_loggers_macros() {
        env_logger::try_init().unwrap_or_default();
        debug!(test_loggers_macros, "message only");
        info!(test_loggers_macros, "message only");
        warn!(test_loggers_macros, "message only");
        critical!(test_loggers_macros, "message only");
        error!(test_loggers_macros, "message only");

        debug!(test_loggers_macros, "KeyValue"; key1 => "value1", key2 => "value2");
        info!(test_loggers_macros, "KeyValue"; key => "value1", key2 => "value2");
        warn!(test_loggers_macros, "KeyValue"; key1 => "value1", key2 => "value2");
        error!(test_loggers_macros, "KeyValue"; key1 => "value1", key2 => "value2");
        critical!(test_loggers_macros, "KeyValue"; key1 => "value1", key2 => "value2");
    }
}
