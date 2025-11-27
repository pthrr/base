#[non_exhaustive]
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),

    #[error(transparent)]
    Lookup(#[from] LookupError),

    #[error("invalid value: {0}")]
    Value(&'static str),

    #[error("parse error: {0}")]
    Parse(&'static str),

    #[error("not implemented: {0}")]
    NotImplemented(&'static str),

    #[error("runtime error: {0}")]
    Runtime(&'static str),
}

#[non_exhaustive]
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticError {
    #[error("matrix is singular or ill-conditioned")]
    Singular,

    #[error("overflow")]
    Overflow,

    #[error("division by zero")]
    ZeroDivision,

    #[error("did not converge")]
    NotConverged,

    #[error("domain error")]
    Domain,
}

#[non_exhaustive]
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    #[error("index out of bounds")]
    Index,

    #[error("key not found")]
    Key,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_arithmetic_error() {
        assert_eq!(
            Error::from(ArithmeticError::Overflow),
            Error::Arithmetic(ArithmeticError::Overflow)
        );
    }

    #[test]
    fn converts_lookup_error() {
        assert_eq!(
            Error::from(LookupError::Key),
            Error::Lookup(LookupError::Key)
        );
    }
}
