//use std::error::Error as StdError;
use std::fmt;
use std::result;


#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum ErrorKind {}

pub struct  Error(Box<ErrorKind>);

pub type Result<T> = result::Result<T, Error>;



impl Error{
    pub fn kind(&self) -> &ErrorKind{
        &self.0
    }

    pub fn into_kind(self) -> ErrorKind{
        *self.0
    }
}

pub(crate) fn new_error(kind: ErrorKind) -> Error {
    Error(Box::new(kind))
}

//todo
// impl StdError for Error{
//     fn cause(&self) -> Option<&dyn StdError> {
//
//     }
// }


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0 {
            _ => write!(f, "{:?}", self.0)
        }
    }
}
