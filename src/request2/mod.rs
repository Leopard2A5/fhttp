use std::path::PathBuf;
use std::str::FromStr;

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::{ErrorKind, Result};
use crate::errors::FhttpError;
use crate::response_handler::{ResponseHandler, JsonPathResponseHandler};

#[derive(Debug)]
pub struct Request2 {
    pub source_path: PathBuf,
    text: String,
    pub dependency: bool,
}

impl Request2 {

    pub fn new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Self {
        Request2 {
            source_path: path.into(),
            text: text.into(),
            dependency: false,
        }
    }

    pub fn depdendency<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Self {
        Request2 {
            source_path: path.into(),
            text: text.into(),
            dependency: true,
        }
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

    pub fn headers(&self) -> Result<HeaderMap> {
        let lines = self.text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with('#'))
            .skip(1)
            .collect::<Vec<&str>>();

        let mut ret = HeaderMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }

            let split: Vec<&str> = line.splitn(2, ':').collect();
            let key = HeaderName::from_bytes(split[0].trim().as_bytes())
                .expect("couldn't create HeaderName");
            let value_text = split[1].trim();
            ret.insert(key, HeaderValue::from_str(value_text).unwrap());
        }

        Ok(ret)
    }

    pub fn body(&self) -> &str {
        let mut body_start = None;
        let mut body_end = None;
        let mut chars: usize = 0;
        let mut last_char = None;

        for (index, chr) in self.text.chars().enumerate() {
            if body_start.is_none() && chr == '\n' && last_char == Some('\n') {
                body_start = Some(chars + 1);
            } else if body_end.is_none() && chr == '%' && &self.text[(index - 4)..index] == "\n> {" {
                body_end = Some(index - 4);
                break;
            }

            last_char = Some(chr);
            chars += 1;
        }

        let body_start = body_start.unwrap();
        let body_end = body_end.unwrap();

        &self.text[body_start..body_end]
    }

    pub fn response_handler(&self) -> Result<Option<Box<dyn ResponseHandler>>> {
        lazy_static! {
            static ref RE_RESPONSE_HANDLER: Regex = Regex::new(r"(?sm)>\s*\{%(.*)%}").unwrap();
        };

        if let Some(captures) = RE_RESPONSE_HANDLER.captures(&self.text) {
            if let Some(group) = captures.get(1) {
                let group = group.as_str().trim();
                let parts: Vec<&str> = group.splitn(2, ' ').collect();
                let kind = parts[0];
                let content = parts[1];

                match kind {
                    "json" => Ok(Some(Box::new(JsonPathResponseHandler::new(content)))),
                    unknown => Err(FhttpError::new(ErrorKind::RequestParseException(format!("Unknown response handler '{}'", unknown))))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
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
    use indoc::indoc;

    use super::*;

    #[test]
    fn method() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.method()?, Method::POST);

        Ok(())
    }

    #[test]
    fn method_no_first_line() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            # POST http://localhost:8080
        "##));

        match req.method() {
            Err(FhttpError { kind: ErrorKind::RequestParseException(ref msg) }) => {
                assert_eq!(msg, "Could not find first line");
            },
            _ => panic!("Expected error!")
        }

        Ok(())
    }

    #[test]
    fn url() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.url()?, "http://localhost:8080");

        Ok(())
    }

    #[test]
    fn headers() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
            # comment
            content-type: application/json; charset=UTF-8
            accept: application/json

            not-a-header: not-a-header-value
        "##));

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json; charset=UTF-8").unwrap());
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json").unwrap());
        assert_eq!(req.headers()?, expected_headers);

        Ok(())
    }

    #[test]
    fn body() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            POST http://localhost:8080

            this is the body

            this as well

            > {%
                json $
            %}
        "##));

        assert_eq!(
            req.body(),
            indoc!(r##"
                this is the body

                this as well
            "##)
        );

        Ok(())
    }

    #[test]
    fn response_handler() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            POST http://localhost:8080

            this is the body

            > {%
                json $
            %}
        "##));

        assert!(req.response_handler()?.is_some());

        Ok(())
    }
}
