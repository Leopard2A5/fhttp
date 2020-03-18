use std::convert::From;

use reqwest::blocking::Client as InnerClient;
use reqwest::{Url, Error};

use crate::{FhttpError, Request, Response};

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn exec(
        &self,
        request: Request
    ) -> Result<Response> {
        let client: InnerClient = InnerClient::new();
        let url = &request.url()?;
        let url = Url::parse(url)
            .map_err(|_| FhttpError::new(format!("Invalid URL: '{}'", url)))?;
        let req = client
            .request(request.method()?, url)
            .headers(request.headers()?)
            .body(request.body()?.into_owned());
        let response = req.send()?;
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().unwrap();

        let body = match request.response_handler()? {
            Some(handler) => {
                handler.process_body(&text)
            },
            None => text
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

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    LocalError(FhttpError),
    RemoteError(String),
}

impl From<FhttpError> for ClientError {
    fn from(e: FhttpError) -> Self {
        ClientError::LocalError(e)
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: Error) -> Self {
        ClientError::RemoteError(
            format!("{}", e)
        )
    }
}
