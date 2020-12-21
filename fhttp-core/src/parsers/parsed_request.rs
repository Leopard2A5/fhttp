use reqwest::header::HeaderMap;
use reqwest::Method;
#[cfg(test)]
use serde_json::Value;

use crate::request::body::Body;
use crate::response_handler::ResponseHandler;

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedRequest {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Body,
    pub response_handler: Option<ResponseHandler>,
}

#[cfg(test)]
impl ParsedRequest {
    pub fn basic(
        method: &'static str,
        url: &'static str
    ) -> Self {
        use std::str::FromStr;

        ParsedRequest {
            method: Method::from_str(method).unwrap(),
            url: url.to_owned(),
            headers: HeaderMap::new(),
            body: Body::Plain(String::new()),
            response_handler: None,
        }
    }

    pub fn add_header(
        mut self,
        name: &'static str,
        value: &'static str,
    ) -> Self {
        use std::str::FromStr;
        use reqwest::header::{HeaderName, HeaderValue};

        self.headers.insert(
            HeaderName::from_str(name).unwrap(),
            HeaderValue::from_str(value).unwrap()
        );

        self
    }

    pub fn body(
        mut self,
        body: &'static str,
    ) -> Self {
        self.body = Body::Plain(body.to_owned());

        self
    }

    pub fn gql_body(
        mut self,
        body: Value,
    ) -> Self {
        self.body = Body::Plain(
            serde_json::to_string(&body).unwrap()
        );

        self
    }

    pub fn response_handler_json(
        mut self,
        handler: &'static str,
    ) -> Self {
        self.response_handler = Some(ResponseHandler::Json { json_path: handler.to_owned(), });

        self
    }
}
