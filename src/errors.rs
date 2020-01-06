use std::io;
use std::fmt::Formatter;
use std::io::Error;

#[derive(Debug)]
pub enum ErrorKind {
    IO(io::Error),
}

#[derive(Debug)]
pub struct FhttpError {
    kind: ErrorKind,
}

impl std::fmt::Display for FhttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "FHTTP error: {:?}", self.kind)
    }
}

impl std::error::Error for FhttpError {
}

impl std::convert::From<std::io::Error> for FhttpError {
    fn from(err: Error) -> Self {
        FhttpError {
            kind: ErrorKind::IO(err)
        }
    }
}
