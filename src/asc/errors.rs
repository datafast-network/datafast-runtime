use crate::bignumber::errors as BNErr;
use thiserror::Error;

#[derive(Debug, Error)]
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
