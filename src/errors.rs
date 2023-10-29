use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostExportErrors {
    #[error("Somethig wrong: {0}")]
    Plain(String),
}

#[derive(Debug, Error)]
pub enum ManifestLoaderError {
    #[error("No datasource with id={0} exists")]
    InvalidDataSource(String),
}

#[derive(Debug, Error)]
pub enum SwrError {
    #[error(transparent)]
    ManifestLoader(#[from] ManifestLoaderError),
}
