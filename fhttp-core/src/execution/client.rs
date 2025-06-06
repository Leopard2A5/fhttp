use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::multipart;
use reqwest::header::HeaderMap;
use reqwest::{Method, Url};

use crate::request::body::{Body, MultipartPart};
use crate::{Response, ResponseHandler};
use crate::postprocessing::response_handler::ResponseHandlerInput;

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
        let url = Url::parse(url).with_context(|| format!("Invalid URL: '{}'", url))?;
        let mut req_builder = client.request(method, url).headers(headers);
        if let Some(timeout) = timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let req_builder = match body {
            Body::Plain(body) => req_builder.body(body),
            Body::Multipart(parts) => {
                let mut multipart = multipart::Form::new();
                for part in parts {
                    match part {
                        MultipartPart::File {
                            name,
                            file_path,
                            mime_str,
                        } => {
                            let path_clone = file_path.clone();
                            let mut tmp =
                                multipart::Part::file(file_path.clone()).with_context(|| {
                                    format!("Error opening file {}", path_clone.to_str())
                                })?;
                            if let Some(mime_str) = mime_str {
                                tmp = tmp.mime_str(&mime_str).with_context(|| {
                                    format!("error parsing mime string '{}'", &mime_str)
                                })?;
                            }
                            multipart = multipart.part(name, tmp);
                        }
                        MultipartPart::Text {
                            name,
                            text,
                            mime_str,
                        } => {
                            let mut tmp = multipart::Part::text(text.clone());
                            if let Some(mime_str) = mime_str {
                                tmp = tmp.mime_str(&mime_str).with_context(|| {
                                    format!("error parsing mime string '{}'", &mime_str)
                                })?;
                            }
                            multipart = multipart.part(name, tmp);
                        }
                    }
                }
                req_builder.multipart(multipart)
            }
        };

        let response = req_builder.send()?;
        let status = response.status();
        let text = response.text()?;
        let response_handler_input = ResponseHandlerInput { status_code: status.as_u16(), body: text };

        let body = match (status.is_success(), response_handler) {
            | (_, Some(handler @ ResponseHandler::Rhai { .. }))
            | (true, Some(handler))
            => handler.process_body(response_handler_input)?,
            _ => response_handler_input.body,
        };
        
        Ok(Response::new(status, body))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock_multipart::prelude::*;

    use crate::request::body::MultipartPart;
    use crate::test_utils::root;

    use super::*;

    #[rstest]
    async fn should_correctly_handle_new_multiparts_async() -> Result<()> {
        let mock_server = MockServer::start().await;
        let image_path = root().join("resources/image.jpg");
        let image_body = std::fs::read(&image_path).unwrap();

        Mock::given(method("POST"))
            .and(NumberOfParts(3))
            .and(
                ContainsPart::new()
                    .with_name("text")
                    .with_body("this is a text part".as_bytes())
                    .with_content_type("text/plain"),
            )
            .and(
                ContainsPart::new()
                    .with_name("textfile")
                    .with_filename("Cargo.toml")
                    .with_content_type("plain/text"),
            )
            .and(
                ContainsPart::new()
                    .with_name("image")
                    .with_content_type("image/jpeg")
                    .with_body(image_body),
            )
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        Client::new().exec(
            Method::POST,
            &mock_server.uri().to_string(),
            HeaderMap::new(),
            Body::Multipart(vec![
                MultipartPart::Text {
                    name: "text".to_string(),
                    text: "this is a text part".to_string(),
                    mime_str: Some("text/plain".to_string()),
                },
                MultipartPart::File {
                    name: "textfile".to_string(),
                    file_path: root().join("Cargo.toml"),
                    mime_str: Some("plain/text".to_string()),
                },
                MultipartPart::File {
                    name: "image".to_string(),
                    file_path: image_path,
                    mime_str: Some("image/jpeg".to_string()),
                },
            ]),
            None,
            None,
        )?;

        Ok(())
    }
}
