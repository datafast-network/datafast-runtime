use std::fs;

use crate::warn;

pub fn welcome() {
    let contents =
        fs::read_to_string("./welcome.txt").expect("Should have been able to read the file");

    warn!(DatafastRuntime, "\nA product of Datafast - [df|runtime]");
    log::info!("\n\n{contents}");
}

#[cfg(test)]
mod test {
    use super::welcome;

    #[test]
    fn test_welcome() {
        env_logger::try_init().unwrap_or_default();
        welcome();
    }
}
