use std::fmt::{self, Display, Formatter};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for FhttpError {}
