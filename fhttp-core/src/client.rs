use reqwest::blocking::multipart;
use reqwest::{Url, Method};

use crate::{FhttpError, Response, Result, ResponseHandler};
use crate::request_def::body::{Body, File};
use reqwest::header::HeaderMap;
use std::time::Duration;
use std::collections::HashMap;

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn exec(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Body,
        response_handler: Option<ResponseHandler>,
        timeout: Option<Duration>,
    ) -> Result<Response> {
        let client = reqwest::blocking::Client::new();
        let url = Url::parse(&url)
            .map_err(|_| FhttpError::new(format!("Invalid URL: '{}'", url)))?;
        let mut req_builder = client
            .request(method, url)
            .headers(headers);
        if let Some(timeout) = timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let req_builder = match body {
            Body::Plain(body) => req_builder.body(body),
            Body::Files(files) => {
                let mut multipart = multipart::Form::new();
                for File { name, path } in files {
                    multipart = multipart.file(name, path.clone())
                        .map_err(|_| FhttpError::new(format!("Error opening file {}", path.to_str())))?;
                }
                req_builder.multipart(multipart)
            },
        };

        let response = req_builder.send()?;
        let status = response.status();
        let headers = response.headers().clone();
        let header_map = headers.iter()
            .map(|(name, value)|
                (name.as_str(), value.to_str().unwrap())
            )
            .collect::<HashMap<_, _>>();
        let text = response.text().unwrap();

        let body = match status.is_success() {
            true => match response_handler {
                Some(handler) => {
                    handler.process_body(status.as_u16(), &header_map, &text)?
                },
                None => text
            },
            false => text,
        };

        Ok(
            Response::new(
                status,
                headers,
                body
            )
        )
    }
}
