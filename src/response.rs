use reqwest::StatusCode;
use reqwest::header::HeaderMap;

#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: String
}

impl Response {
    pub fn new <S: Into<String>> (
        status: StatusCode,
        headers: HeaderMap,
        body: S
    ) -> Self {
        Response {
            status,
            headers,
            body: body.into()
        }
    }

    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}
