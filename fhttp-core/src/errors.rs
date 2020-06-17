use std::convert::From;
use std::fmt::{self, Display, Formatter};
use reqwest::Error;

#[derive(Debug, Eq, PartialEq)]
pub struct FhttpError {
    pub msg: String,
}

impl FhttpError {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        FhttpError {
            msg: msg.into(),
        }
    }
}

impl Display for FhttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for FhttpError {}

impl From<reqwest::Error> for FhttpError {
    fn from(e: Error) -> Self {
        FhttpError::new(format!("{}", e))
    }
}

pub type Result<T> = std::result::Result<T, FhttpError>;
