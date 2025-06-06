use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use serde::Deserialize;

use crate::path_utils::{CanonicalizedPathBuf, RelativePath};
use crate::request::body::{Body, MultipartPart};
use crate::request::Request;
use crate::ResponseHandler;

#[derive(Debug, Deserialize)]
struct StructuredRequestSource {
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    response_handler: Option<StructuredResponseHandler>,
    body: Option<StructuredBody>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StructuredBody {
    Plain(String),
    Mutlipart(Vec<StructuredMultipartPart>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StructuredMultipartPart {
    Text {
        name: String,
        text: String,
        mime: Option<String>,
    },
    File {
        name: String,
        filepath: String,
        mime: Option<String>,
    },
}

impl StructuredMultipartPart {
    fn into_part(self, reference_location: &CanonicalizedPathBuf) -> Result<MultipartPart> {
        Ok(match self {
            StructuredMultipartPart::Text {
                name,
                text,
                mime: mime_str,
            } => MultipartPart::Text {
                name,
                text,
                mime_str,
            },
            StructuredMultipartPart::File {
                name,
                filepath: file_path,
                mime: mime_str,
            } => MultipartPart::File {
                name,
                file_path: reference_location.get_dependency_path(&file_path)?,
                mime_str,
            },
        })
    }
}

impl StructuredBody {
    pub fn into_body(self, reference_location: &CanonicalizedPathBuf) -> Result<Body> {
        match self {
            StructuredBody::Plain(text) => Ok(Body::Plain(text.to_string())),
            StructuredBody::Mutlipart(parts) => Ok(Body::Multipart(
                parts
                    .into_iter()
                    .map(|it| it.into_part(reference_location))
                    .collect::<Result<Vec<MultipartPart>>>()?,
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StructuredResponseHandler {
    pub json: Option<String>,
    pub deno: Option<String>,
    pub rhai: Option<String>,
}

impl StructuredResponseHandler {
    pub fn response_handler(self) -> Option<ResponseHandler> {
        if let Some(json) = self.json {
            Some(ResponseHandler::Json { json_path: json })
        } else if let Some(program) = self.deno {
            Some(ResponseHandler::Deno { program })
        } else if let Some(program) = self.rhai {
            Some(ResponseHandler::Rhai { program })
        } else {
            None
        }
    }
}

impl TryFrom<(&CanonicalizedPathBuf, StructuredRequestSource)> for Request {
    type Error = anyhow::Error;

    fn try_from(
        arg: (&CanonicalizedPathBuf, StructuredRequestSource),
    ) -> std::result::Result<Self, Self::Error> {
        let reference_location = arg.0;
        let value = arg.1;

        let headers = match value.headers {
            Some(headers) => {
                let mut tmp = HeaderMap::new();
                for (name, value) in headers {
                    tmp.append(HeaderName::from_str(&name)?, HeaderValue::from_str(&value)?);
                }
                Ok::<HeaderMap, anyhow::Error>(tmp)
            }
            None => Ok(HeaderMap::new()),
        }?;

        Ok(Request {
            method: Method::from_str(&value.method)?,
            url: value.url.to_string(),
            headers,
            body: value
                .body
                .map(|it| it.into_body(reference_location))
                .unwrap_or(Ok(Body::Plain("".to_string())))?,
            response_handler: value
                .response_handler
                .and_then(StructuredResponseHandler::response_handler),
        })
    }
}

pub fn parse_request_from_json(
    reference_location: &CanonicalizedPathBuf,
    text: &str,
) -> Result<Request> {
    let structured = serde_json::from_str(text)?;
    Request::try_from((reference_location, structured))
}

pub fn parse_request_from_yaml(
    reference_location: &CanonicalizedPathBuf,
    text: &str,
) -> Result<Request> {
    let structured = serde_yaml::from_str(text)?;
    Request::try_from((reference_location, structured))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    use crate::request::body::MultipartPart;
    use crate::test_utils::root;
    use crate::ResponseHandler;

    use super::*;

    #[test]
    fn should_parse_minimal_json_request() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo"
            }
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_headers() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "headers": {
                    "accept": "application/json"
                }
            }
        "#},
        )?;

        let headers = {
            let mut tmp = HeaderMap::new();
            tmp.append(
                HeaderName::from_str("accept").unwrap(),
                HeaderValue::from_str("application/json").unwrap(),
            );
            tmp
        };

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers,
                body: Body::Plain("".to_string()),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_json_response_handler() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "response_handler": {
                    "json": "$.data"
                }
            }
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: Some(ResponseHandler::Json {
                    json_path: "$.data".to_string()
                }),
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_deno_response_handler() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "response_handler": {
                    "deno": "setResult('ok!');"
                }
            }
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: Some(ResponseHandler::Deno {
                    program: "setResult('ok!');".to_string()
                }),
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_plain_body() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "body": "plain body"
            }
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("plain body".to_string()),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_multipart_body() -> Result<()> {
        let result = parse_request_from_json(
            &root(),
            indoc! {r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "body": [
                    {
                        "name": "textpart1",
                        "text": "text for part 1"
                    },
                    {
                        "name": "textpart2",
                        "text": "text for part 2",
                        "mime": "text/plain"
                    },
                    {
                        "name": "filepart1",
                        "filepath": "resources/image.jpg"
                    },
                    {
                        "name": "filepart2",
                        "filepath": "resources/image.jpg",
                        "mime": "image/png"
                    }
                ]
            }
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Multipart(vec![
                    MultipartPart::Text {
                        name: "textpart1".to_string(),
                        text: "text for part 1".to_string(),
                        mime_str: None,
                    },
                    MultipartPart::Text {
                        name: "textpart2".to_string(),
                        text: "text for part 2".to_string(),
                        mime_str: Some("text/plain".to_string()),
                    },
                    MultipartPart::File {
                        name: "filepart1".to_string(),
                        file_path: root().join("resources/image.jpg"),
                        mime_str: None,
                    },
                    MultipartPart::File {
                        name: "filepart2".to_string(),
                        file_path: root().join("resources/image.jpg"),
                        mime_str: Some("image/png".to_string()),
                    },
                ]),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_full_yaml_request() -> Result<()> {
        let result = parse_request_from_yaml(
            &root(),
            indoc! {r#"
            method: POST
            url: http://localhost/foo
            body: hello there
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("hello there".to_string()),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_full_yaml_multipart_request() -> Result<()> {
        let result = parse_request_from_yaml(
            &root(),
            indoc! {r#"
            method: POST
            url: http://localhost/foo
            headers:
                accept: application/json
            body:
                - name: textpart1
                  text: text for part 1
                - name: textpart2
                  text: text for part 2
                  mime: text/plain
                - name: filepart1
                  filepath: resources/image.jpg
                - name: filepart2
                  filepath: resources/image.jpg
                  mime: image/png
        "#},
        )?;

        let headers = {
            let mut tmp = HeaderMap::new();
            tmp.append(
                HeaderName::from_str("accept").unwrap(),
                HeaderValue::from_str("application/json").unwrap(),
            );
            tmp
        };

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers,
                body: Body::Multipart(vec![
                    MultipartPart::Text {
                        name: "textpart1".to_string(),
                        text: "text for part 1".to_string(),
                        mime_str: None,
                    },
                    MultipartPart::Text {
                        name: "textpart2".to_string(),
                        text: "text for part 2".to_string(),
                        mime_str: Some("text/plain".to_string()),
                    },
                    MultipartPart::File {
                        name: "filepart1".to_string(),
                        file_path: root().join("resources/image.jpg"),
                        mime_str: None,
                    },
                    MultipartPart::File {
                        name: "filepart2".to_string(),
                        file_path: root().join("resources/image.jpg"),
                        mime_str: Some("image/png".to_string()),
                    },
                ]),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_request_with_rhai_response_handler() -> Result<()> {
        let result = parse_request_from_yaml(
            &root(),
            indoc! {r#"
            method: GET
            url: http://localhost/foo
            response_handler:
              rhai: program
        "#},
        )?;

        assert_eq!(
            result,
            Request {
                method: Method::GET,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: Some(ResponseHandler::Rhai { program: "program".to_string() })
            }
        );

        Ok(())
    }
}
