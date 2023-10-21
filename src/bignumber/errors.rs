use thiserror::Error;

#[derive(Error, Debug)]
pub enum BigIntOutOfRangeError {
    #[error("Cannot convert negative BigInt into type")]
    Negative,
    #[error("BigInt value is too large for type")]
    Overflow,
}

#[derive(Error, Debug)]
pub enum BigNumberErr {
    #[error("Parser Error")]
    Parser,
    #[error(transparent)]
    OutOfRange(#[from] BigIntOutOfRangeError),
    #[error("Number too big")]
    NumberTooBig,
}
