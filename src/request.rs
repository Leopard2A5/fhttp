use std::path::{Path, PathBuf};
use std::str::{FromStr, Lines};
use std::fs;
use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::{Result, FhttpError};
use crate::errors::ErrorKind;
use crate::response_handler::{JsonPathResponseHandler, ResponseHandler};

use serde_json::{Value, Map};

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: String,
    pub source_path: PathBuf,
    pub response_handler: Option<Box<dyn ResponseHandler>>,
    pub dependency: bool,
}

impl Request {

    pub fn parse(
        input: String,
        path: &Path
    ) -> Request {
        Self::_parse(input, path, false)
    }

    pub fn parse_dependency(
        input: String,
        path: &Path
    ) -> Self {
        Self::_parse(input, path, true)
    }

    fn _parse(
        input: String,
        path: &Path,
        dependency: bool
    ) -> Request {
        let parts = split_body_parts(input.lines());
        let (method, url, headers) = parse_header_part(&parts[0]);

        let (body, response_handler) = match parts.len() {
            1 => (String::new(), None),
            2 => {
                match parse_response_handler_script(&parts[1]) {
                    None => (parse_body(&parts[1]), None),
                    tmp => (String::new(), tmp)
                }
            },
            _ => (parse_body(&parts[1]), parse_response_handler_script(parts.last().unwrap()))
        };

        Request {
            method,
            url,
            headers,
            body,
            source_path: fs::canonicalize(path).unwrap(),
            response_handler,
            dependency
        }
    }

    fn parse_gql(
        input: String,
        path: &Path,
        dependency: bool
    ) -> Result<Request> {
        let parts = split_body_parts(input.lines());
        let (method, url, headers) = parse_header_part(&parts[0]);

        let (query, variables, response_handler) = match parts.len() {
            1 => return Err(FhttpError::new(ErrorKind::RequestParseException("graphql requests need a body".into()))),
            2 => (parse_body(&parts[1]), Value::Object(Map::new()), None),
            3 => {
                if let Some(resp_handler) = parse_response_handler_script(&parts[2]) {
                    (parse_body(&parts[1]), Value::Object(Map::new()), Some(resp_handler))
                } else {
                    (parse_body(&parts[1]), parse_variables(&parts[2]), None)
                }
            },
            4 => {
                (parse_body(&parts[1]), parse_variables(&parts[2]), parse_response_handler_script(&parts[3]))
            },
            _ => return Err(FhttpError::new(ErrorKind::RequestParseException("graphql requests don't support multipart".into())))
        };

        let query = Value::String(query.clone());
        let mut map = Map::new();
        map.insert("query".into(), query);
        map.insert("variables".into(), variables);
        let body = Value::Object(map);

        Ok(
            Request {
                method,
                url,
                headers,
                body: serde_json::to_string_pretty(&body).unwrap(),
                source_path: fs::canonicalize(path).unwrap(),
                response_handler,
                dependency
            }
        )
    }

}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.source_path == other.source_path
    }
}

impl Eq for Request {}

impl Hash for Request {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_path.hash(state);
    }
}

fn split_body_parts(lines: Lines) -> Vec<Vec<String>> {
    let mut ret = vec![vec![]];

    for line in lines {
        if line.trim().is_empty() {
            ret.push(vec![]);
        } else {
            ret.last_mut().unwrap().push(line.to_owned());
        }
    }

    ret.into_iter()
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_header_part(lines: &Vec<String>) -> (Method, String, HeaderMap) {
    let mut lines: Vec<String> = lines.into_iter()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim().starts_with('#'))
        .map(|line| line.to_owned())
        .collect();
    let first_line = lines.remove(0);
    let split: Vec<&str> = first_line.splitn(2, ' ').collect();
    let method_string = split[0];
    let url = split[1].to_owned();
    let method = Method::from_str(method_string)
        .expect(&format!("couldn't parse method '{}'", method_string));

    let mut headers = HeaderMap::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            break;
        }

        let split: Vec<&str> = line.splitn(2, ':').collect();
        let key = HeaderName::from_bytes(split[0].as_bytes())
            .expect("couldn't create HeaderName");
        let value_text = split[1].trim();
        headers.insert(key, HeaderValue::from_str(value_text).unwrap());
    }

    return (
        method,
        url,
        headers
    );
}

fn parse_body(lines: &Vec<String>) -> String {
    lines.join("\n")
}

fn parse_response_handler_script(lines: &Vec<String>) -> Option<Box<dyn ResponseHandler>> {
    lazy_static! {
        static ref RE_RESPONSE_HANDLER: Regex = Regex::new(r"(?sm)>\s*\{%(.*)%}").unwrap();
        static ref RE_TRIM_LINES: Regex = Regex::new(r"\s*\n\s*").unwrap();
    };
    let text = lines.join("\n");

    if let Some(captures) = RE_RESPONSE_HANDLER.captures(&text) {
        if let Some(group) = captures.get(1) {
            let group = group.as_str().trim();
            let parts: Vec<&str> = group.splitn(2, ' ').collect();
            let kind = parts[0];
            let content = parts[1];

            match kind {
                "json" => Some(Box::new(JsonPathResponseHandler::new(content))),
                _ => None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_variables(lines: &Vec<String>) -> Value {
    let text = lines.join("\n");
    serde_json::from_str::<Value>(&text)
        .or_else::<FhttpError, _>(|_| Ok(Value::Object(Map::new())))
        .unwrap()
}

#[cfg(test)]
mod test {
    use indoc::indoc;
    use super::*;

    #[test]
    fn should_parse_full_request() {
        let input = indoc!("
            GET https://google.com
            x-request-id: abc
            content-type: application/json; charset=UTF-8
            accept: application/json, application/xml

            body1
            body2

            > {%
                json $
            %}
        ").to_owned();

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert(HeaderName::from_str("x-request-id").unwrap(), HeaderValue::from_str("abc").unwrap());
        expected_headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json; charset=UTF-8").unwrap());
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json, application/xml").unwrap());

        let result = Request::parse(input, &std::env::current_dir().unwrap());
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.url, "https://google.com");
        assert_eq!(result.headers, expected_headers);
        assert_eq!(result.body, "body1\nbody2");
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        if let Some(_) = result.response_handler {
            // TODO check the actual response handler impl
        } else {
            panic!("expected a response handler");
        }
    }

    #[test]
    fn should_canonicalize_path() {
        let original_path = PathBuf::from_str("./src/../.").unwrap();
        let req = Request::parse("GET http://localhost".into(), &original_path);
        let canonicalized_path = std::fs::canonicalize(&original_path).unwrap();

        assert_eq!(req.source_path, canonicalized_path);
    }

    #[test]
    fn should_parse_request_with_headers_and_handler() {
        let input = indoc!("
            GET https://google.com

            > {%
                json $.foo
            %}

        ").to_owned();

        let result = Request::parse(input, &std::env::current_dir().unwrap());
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.url, "https://google.com");
        assert_eq!(result.headers, HeaderMap::new());
        assert_eq!(result.body, "");
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        if let Some(_) = result.response_handler {
            // TODO check the actual response handler impl
        } else {
            panic!("expected a response handler");
        }
    }

}

#[cfg(test)]
mod parse_gql {
    use super::*;
    use reqwest::Method;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use indoc::indoc;
    use serde_json::json;

    #[test]
    fn parse_gql_with_query_variables_response_handler() {
        let input = indoc!(r##"
            POST http://server:8080/graphql
            Authorization: Bearer token

            query($var: String!) {
                entity(id: $var, foo: "bar") {
                    field1
                    field2
                }
            }

            {
                "var": "entity-id"
            }

            > {%
                json $
            %}
        "##).to_owned();

        let result = Request::parse_gql(input, &std::env::current_dir().unwrap(), false).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());

        let body = serde_json::to_string_pretty(
            &json!({
                "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
                "variables": {
                    "var": "entity-id"
                }
            })
        ).unwrap();

        assert_eq!(result.method, Method::POST);
        assert_eq!(result.url, "http://server:8080/graphql");
        assert_eq!(result.headers, headers);
        assert_eq!(result.body, body);
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        assert_eq!(result.dependency, false);
        assert!(result.response_handler.is_some());
    }

    #[test]
    fn parse_gql_with_query_variables() {
        let input = indoc!(r##"
            POST http://server:8080/graphql
            Authorization: Bearer token

            query($var: String!) {
                entity(id: $var, foo: "bar") {
                    field1
                    field2
                }
            }

            {
                "var": "entity-id"
            }
        "##).to_owned();

        let result = Request::parse_gql(input, &std::env::current_dir().unwrap(), false).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());

        let body = serde_json::to_string_pretty(
            &json!({
                "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
                "variables": {
                    "var": "entity-id"
                }
            })
        ).unwrap();

        assert_eq!(result.method, Method::POST);
        assert_eq!(result.url, "http://server:8080/graphql");
        assert_eq!(result.headers, headers);
        assert_eq!(result.body, body);
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        assert_eq!(result.dependency, false);
        assert!(result.response_handler.is_none());
    }

    #[test]
    fn parse_gql_with_query_response_handler() {
        let input = indoc!(r##"
            POST http://server:8080/graphql
            Authorization: Bearer token

            query($var: String!) {
                entity(id: $var, foo: "bar") {
                    field1
                    field2
                }
            }

            > {%
                json $
            %}
        "##).to_owned();

        let result = Request::parse_gql(input, &std::env::current_dir().unwrap(), false).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());

        let body = serde_json::to_string_pretty(
            &json!({
                "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
                "variables": {}
            })
        ).unwrap();

        assert_eq!(result.method, Method::POST);
        assert_eq!(result.url, "http://server:8080/graphql");
        assert_eq!(result.headers, headers);
        assert_eq!(result.body, body);
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        assert_eq!(result.dependency, false);
        assert!(result.response_handler.is_some());
    }

    #[test]
    fn parse_gql_with_query() {
        let input = indoc!(r##"
            POST http://server:8080/graphql
            Authorization: Bearer token

            query($var: String!) {
                entity(id: $var, foo: "bar") {
                    field1
                    field2
                }
            }
        "##).to_owned();

        let result = Request::parse_gql(input, &std::env::current_dir().unwrap(), false).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());

        let body = serde_json::to_string_pretty(
            &json!({
                "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
                "variables": {}
            })
        ).unwrap();

        assert_eq!(result.method, Method::POST);
        assert_eq!(result.url, "http://server:8080/graphql");
        assert_eq!(result.headers, headers);
        assert_eq!(result.body, body);
        assert_eq!(result.source_path, std::env::current_dir().unwrap());
        assert_eq!(result.dependency, false);
        assert!(result.response_handler.is_none());
    }
}
