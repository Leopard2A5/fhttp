use std::io;
use std::fmt::{self, Display, Formatter};
use std::convert::From;
use reqwest::header::{ToStrError, InvalidHeaderValue};
use serde_json::Error;

#[derive(Debug)]
pub enum ErrorKind {
    IO(io::Error),
    MissingEnvVar(String),
    StringEncodingError,
    RequestParseException(String),
    JsonDeserializationError(String),
}

#[derive(Debug)]
pub struct FhttpError {
    kind: ErrorKind,
}

impl FhttpError {
    pub fn new(kind: ErrorKind) -> Self {
        FhttpError {
            kind
        }
    }
}

impl Display for FhttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "FHTTPerror: {:?}", self.kind)
    }
}

impl std::error::Error for FhttpError {}

impl From<io::Error> for FhttpError {
    fn from(err: io::Error) -> Self {
        FhttpError {
            kind: ErrorKind::IO(err)
        }
    }
}

impl From<ToStrError> for FhttpError {
    fn from(_: ToStrError) -> Self {
        FhttpError::new(ErrorKind::StringEncodingError)
    }
}

impl From<InvalidHeaderValue> for FhttpError {
    fn from(_: InvalidHeaderValue) -> Self {
        FhttpError::new(ErrorKind::RequestParseException("Invalid header value".to_string()))
    }
}

impl From<serde_json::Error> for FhttpError {
    fn from(err: serde_json::Error) -> Self {
        FhttpError::new(ErrorKind::JsonDeserializationError(err.to_string()))
    }
}
