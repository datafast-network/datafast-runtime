use crate::runtime::bignumber::errors as BNErr;
use log::error;
use std::fmt;
use wasmer::RuntimeError;

#[derive(Debug, thiserror::Error)]
pub enum AscError {
    #[error("Size not fit")]
    SizeNotFit,
    #[error("Value overflow: {0}")]
    Overflow(u32),
    #[error("Error: {0}")]
    Plain(String),
    #[error("Bad boolean value: {0}")]
    IncorrectBool(usize),
    #[error("Size does not match")]
    SizeNotMatch,
    #[error("Maximum Recursion Depth reached!")]
    MaxRecursion,
    #[error(transparent)]
    BigNumberOutOfRange(#[from] BNErr::BigNumberErr),
}

#[derive(Debug)]
pub enum DeterministicHostError {
    Gas(anyhow::Error),
    Other(anyhow::Error),
}

impl DeterministicHostError {
    pub fn gas(e: anyhow::Error) -> Self {
        DeterministicHostError::Gas(e)
    }

    pub fn inner(self) -> anyhow::Error {
        match self {
            DeterministicHostError::Gas(e) | DeterministicHostError::Other(e) => e,
        }
    }
}

impl fmt::Display for DeterministicHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeterministicHostError::Gas(e) | DeterministicHostError::Other(e) => e.fmt(f),
        }
    }
}

impl From<anyhow::Error> for DeterministicHostError {
    fn from(e: anyhow::Error) -> DeterministicHostError {
        DeterministicHostError::Other(e)
    }
}

impl From<AscError> for RuntimeError {
    fn from(err: AscError) -> Self {
        RuntimeError::new(err.to_string())
    }
}

impl std::error::Error for DeterministicHostError {}

#[derive(thiserror::Error, Debug)]
pub enum HostExportError {
    #[error("{0:#}")]
    Unknown(#[from] anyhow::Error),

    #[error("{0:#}")]
    PossibleReorg(anyhow::Error),

    #[error("{0:#}")]
    Deterministic(anyhow::Error),
}

impl From<DeterministicHostError> for HostExportError {
    fn from(value: DeterministicHostError) -> Self {
        match value {
            // Until we are confident on the gas numbers, gas errors are not deterministic
            DeterministicHostError::Gas(e) => HostExportError::Unknown(e),
            DeterministicHostError::Other(e) => HostExportError::Deterministic(e),
        }
    }
}
