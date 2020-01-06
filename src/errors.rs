use std::io;
use std::fmt::Formatter;
use std::io::Error;
use std::convert::From;

#[derive(Debug)]
pub enum ErrorKind {
    IO(io::Error),
    MissingEnvVar(String),
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

impl std::fmt::Display for FhttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "FHTTPerror: {:?}", self.kind)
    }
}

impl std::error::Error for FhttpError {}

impl From<std::io::Error> for FhttpError {
    fn from(err: Error) -> Self {
        FhttpError {
            kind: ErrorKind::IO(err)
        }
    }
}
