use std::str::FromStr;

use reqwest::Method;

use crate::{ErrorKind, Result};
use crate::errors::FhttpError;

#[derive(Debug)]
pub struct Request2 {
    text: String,
}

impl Request2 {

    pub fn new<T: Into<String>>(text: T) -> Self {
        Request2 { text: text.into() }
    }

    pub fn method(&self) -> Result<Method> {
        let first_line = self.first_line()?;
        let split: Vec<&str> = first_line.splitn(2, ' ').collect();
        let method_string = split[0];

        Method::from_str(method_string)
            .map_err(|_| FhttpError::new(ErrorKind::RequestParseException(format!("Couldn't parse method '{}'", method_string))))
    }

    pub fn url(&self) -> Result<&str> {
        let first_line = self.first_line()?;
        let mut split: Vec<&str> = first_line.splitn(2, ' ').collect();

        split.pop()
            .ok_or(FhttpError::new(ErrorKind::RequestParseException("Malformed url line".into())))
    }

    fn first_line(&self) -> Result<&str> {
        self.text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with("#"))
            .nth(0)
            .ok_or(FhttpError::new(ErrorKind::RequestParseException("Could not find first line".into())))
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    fn method() -> Result<()> {
        let req = Request2::new(indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.method()?, Method::POST);

        Ok(())
    }

    #[test]
    fn method_no_first_line() {
        let req = Request2::new(indoc!(r##"
            # comment
            # POST http://localhost:8080
        "##));

        match req.method() {
            Err(FhttpError { kind: ErrorKind::RequestParseException(ref msg) }) => {
                assert_eq!(msg, "Could not find first line");
            },
            _ => panic!("Expected error!")
        }
    }

    #[test]
    fn url() -> Result<()> {
        let req = Request2::new(indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.url()?, "http://localhost:8080");

        Ok(())
    }

}
