use reqwest::blocking::Client as InnerClient;
use reqwest::Url;
use crate::{Request, Response};

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn exec(
        &self,
        request: Request
    ) -> Response {
        let client: InnerClient = InnerClient::new();
        let url = Url::parse(&request.url).unwrap();
        let req = client
            .request(request.method, url)
            .headers(request.headers)
            .body(request.body);
        let response = req.send().unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().unwrap();

        let body = match request.response_handler {
            Some(handler) => {
                handler.process_body(&text)
            },
            None => text
        };

        Response::new(
            status,
            headers,
            body
        )
    }
}
