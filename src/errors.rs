use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostExportErrors {
    #[error("Somethig wrong: {0}")]
    Plain(String),
}
