use reqwest::StatusCode;

#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    body: String
}

impl Response {
    pub fn new <S: Into<String>> (
        status: StatusCode,
        body: S
    ) -> Self {
        Response {
            status,
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
