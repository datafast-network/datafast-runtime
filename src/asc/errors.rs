pub enum AscError {
    SizeNotFit,
    Overflow(u32),
    Plain(String),
    IncorrectBool(usize),
    SizeNotMatch,
    MaxRecursion,
}
