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
struct StructuredRequestSource<'a> {
    method: &'a str,
    url: &'a str,
    headers: Option<HashMap<&'a str, &'a str>>,
    response_handler: Option<StructuredResponseHandler<'a>>,
    body: Option<StructuredBody<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StructuredBody<'a> {
    Plain(&'a str),
    Mutlipart(Vec<StructuredMultipartPart<'a>>)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StructuredMultipartPart<'a> {
    Text {
        name: &'a str,
        text: &'a str,
        mime: Option<&'a str>,
    },
    File {
        name: &'a str,
        filepath: &'a str,
        mime: Option<&'a str>,
    },
}

impl<'a> StructuredMultipartPart<'a> {
    fn to_part(self, reference_location: &CanonicalizedPathBuf) -> Result<MultipartPart> {
        Ok(
            match self {
                StructuredMultipartPart::Text { name, text, mime: mime_str } => MultipartPart::Text {
                    name: name.to_string(),
                    text: text.to_string(),
                    mime_str: mime_str.map(|it| it.to_string()),
                },
                StructuredMultipartPart::File { name, filepath: file_path, mime: mime_str } => MultipartPart::File {
                    name: name.to_string(),
                    file_path: reference_location.get_dependency_path(file_path)?,
                    mime_str: mime_str.map(|it| it.to_string()),
                }
            }
        )
    }
}

impl<'a> StructuredBody<'a> {
    pub fn to_body(self, reference_location: &CanonicalizedPathBuf) -> Result<Body> {
        match self {
            StructuredBody::Plain(text) => Ok(Body::Plain(text.to_string())),
            StructuredBody::Mutlipart(parts) => Ok(
                Body::Multipart(
                    parts.into_iter()
                        .map(|it| it.to_part(reference_location))
                        .collect::<Result<Vec<MultipartPart>>>()?
                )
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
struct StructuredResponseHandler<'a> {
    pub json: Option<&'a str>,
    pub deno: Option<&'a str>,
}

impl<'a> StructuredResponseHandler<'a> {
    pub fn response_handler(self) -> Option<ResponseHandler> {
        if let Some(json) = self.json {
            Some(ResponseHandler::Json { json_path: json.to_string() })
        } else if let Some(code) = self.deno {
            Some(ResponseHandler::Deno { program: code.to_string() })
        } else {
            None
        }
    }
}

impl<'a> TryFrom<&'a str> for StructuredRequestSource<'a> {
    type Error = serde_json::Error;

    fn try_from(value: &'a str) -> std::result::Result<Self, Self::Error> {
        serde_json::from_str(value)
    }
}

impl<'a> TryFrom<(&CanonicalizedPathBuf, StructuredRequestSource<'a>)> for Request {
    type Error = anyhow::Error;

    fn try_from(arg: (&CanonicalizedPathBuf ,StructuredRequestSource<'a>)) -> std::result::Result<Self, Self::Error> {
        let reference_location = arg.0;
        let value = arg.1;

        let headers = match value.headers {
            Some(headers) => {
                let mut tmp = HeaderMap::new();
                for (name, value) in headers {
                    tmp.append(
                        HeaderName::from_str(name)?,
                        HeaderValue::from_str(value)?
                    );
                }
                Ok::<HeaderMap, anyhow::Error>(tmp)
            },
            None => Ok(HeaderMap::new()),
        }?;

        Ok(
            Request {
                method: Method::from_str(value.method)?,
                url: value.url.to_string(),
                headers,
                body: value.body
                    .map(|it| it.to_body(reference_location))
                    .unwrap_or(Ok(Body::Plain("".to_string())))?,
                response_handler: value.response_handler
                    .and_then(StructuredResponseHandler::response_handler),
            }
        )
    }
}

pub fn parse_request_from_json(
    reference_location: &CanonicalizedPathBuf,
    text: &str
) -> Result<Request> {
    let structured = StructuredRequestSource::try_from(text)?;
    Request::try_from((reference_location, structured))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    use crate::request::body::MultipartPart;
    use crate::ResponseHandler;
    use crate::test_utils::root;

    use super::*;

    #[test]
    fn should_parse_minimal_json_request() -> Result<()> {
        let result = parse_request_from_json(&root(), indoc!{r#"
            {
                "method": "POST",
                "url": "http://localhost/foo"
            }
        "#})?;

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
        let result = parse_request_from_json(&root(), indoc!{r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "headers": {
                    "accept": "application/json"
                }
            }
        "#})?;

        let headers = {
            let mut tmp = HeaderMap::new();
            tmp.append(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json").unwrap());
            tmp
        };

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: headers,
                body: Body::Plain("".to_string()),
                response_handler: None
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_json_response_handler() -> Result<()> {
        let result = parse_request_from_json(&root(), indoc!{r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "response_handler": {
                    "json": "$.data"
                }
            }
        "#})?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: Some(ResponseHandler::Json { json_path: "$.data".to_string() }),
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_deno_response_handler() -> Result<()> {
        let result = parse_request_from_json(&root(), indoc!{r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "response_handler": {
                    "deno": "setResult('ok!');"
                }
            }
        "#})?;

        assert_eq!(
            result,
            Request {
                method: Method::POST,
                url: "http://localhost/foo".to_string(),
                headers: HeaderMap::new(),
                body: Body::Plain("".to_string()),
                response_handler: Some(ResponseHandler::Deno { program: "setResult('ok!');".to_string() }),
            }
        );

        Ok(())
    }

    #[test]
    fn should_parse_json_request_with_plain_body() -> Result<()> {
        let result = parse_request_from_json(&root(), indoc!{r#"
            {
                "method": "POST",
                "url": "http://localhost/foo",
                "body": "plain body"
            }
        "#})?;

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
        let result = parse_request_from_json(&root(), indoc!{r#"
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
        "#})?;

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
}
