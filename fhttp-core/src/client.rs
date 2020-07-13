use reqwest::Url;
use reqwest::blocking::multipart;

use crate::{Result, FhttpError, Response, Request, RequestResponseHandlerExt};
use crate::request::body::Body;
use crate::request::has_body::HasBody;

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn exec(
        &self,
        request: Request
    ) -> Result<Response> {
        let client = reqwest::blocking::Client::new();
        let url = request.url()?;
        let url = Url::parse(url)
            .map_err(|_| FhttpError::new(format!("Invalid URL: '{}'", url)))?;
        let req_body = request.body()?;
        let req_builder = client
            .request(request.method()?, url)
            .headers(request.headers()?);

        let req_builder = match req_body {
            Body::Plain(body) => req_builder.body(body.into_owned()),
            Body::File { name, path } => unimplemented!(),
        };

        let response = req_builder.send()?;
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
