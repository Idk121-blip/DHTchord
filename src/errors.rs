use std::fmt;
use std::result;
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum PutError {
    ForwardingRequest(String),
    ErrorStoringFile,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum GetError {
    ForwardingRequest(String),
    ErrorRetrievingFile,
    NotFound,
    HexConversion,
}

pub struct Error(Box<PutError>);

pub type Result<T> = result::Result<T, Error>;

impl Error {
    pub const fn kind(&self) -> &PutError {
        &self.0
    }

    pub fn into_kind(self) -> PutError {
        *self.0
    }
}

pub(crate) fn new_error(kind: PutError) -> Error {
    Error(Box::new(kind))
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
