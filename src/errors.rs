#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum PutError {
    ForwardingRequest(String),
    ErrorStoringFile,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum GetError {
    ForwardingRequest(String),
    ErrorRetrievingFile,
    NotFound,
    HexConversion,
}
