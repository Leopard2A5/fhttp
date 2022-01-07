use reqwest::header::HeaderMap;
use reqwest::Method;
#[cfg(test)] use serde_json::Value;
#[cfg(test)] use body::MultipartPart;

use body::Body;
use crate::postprocessing::response_handler::ResponseHandler;

pub mod body;

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Body,
    pub response_handler: Option<ResponseHandler>,
}

#[cfg(test)]
impl Request {
    pub fn basic(
        method: &'static str,
        url: &'static str
    ) -> Self {
        use std::str::FromStr;

        Request {
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

    pub fn file_body(
        mut self,
        files: &[(&str, &str)],
    ) -> Self {
        use body::File;
        use crate::test_utils::root;

        self.body = Body::Files(
            files.into_iter()
                .map(|it| (it.0.to_owned(), it.1))
                .map(|(name, path)| (name, root().join(path)))
                .map(|(name, path)| File { name, path })
                .collect()
        );

        self
    }

    pub fn multipart(
        mut self,
        parts: &[MultipartPart],
    ) -> Self {
        self.body = Body::Multipart(parts.to_vec());

        self
    }

    pub fn response_handler_json(
        mut self,
        handler: &'static str,
    ) -> Self {
        self.response_handler = Some(ResponseHandler::Json { json_path: handler.to_owned(), });

        self
    }

    pub fn response_handler_deno(
        mut self,
        handler: &'static str,
    ) -> Self {
        self.response_handler = Some(ResponseHandler::Deno { program: handler.to_owned(), });

        self
    }
}
