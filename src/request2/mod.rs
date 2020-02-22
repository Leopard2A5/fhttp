use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use serde_json::map::Map;
use serde_json::Value;

use crate::{ErrorKind, Result};
use crate::errors::FhttpError;
use crate::request_preprocessor2::get_dependency_path;
use crate::response_handler::{JsonPathResponseHandler, ResponseHandler};

lazy_static!{
    pub static ref RE_REQUEST: Regex = Regex::new(r#"(?m)\$\{request\("([^"]+)"\)}"#).unwrap();
}

#[derive(Debug, Eq)]
pub struct Request2 {
    pub source_path: PathBuf,
    pub text: String,
    pub dependency: bool,
}

impl Request2 {

    pub fn new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Self {
        Request2 {
            source_path: path.into(),
            text: text.into(),
            dependency: false,
        }
    }

    pub fn depdendency<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Self {
        Request2 {
            source_path: path.into(),
            text: text.into(),
            dependency: true,
        }
    }

    pub fn from_file(
        path: &Path,
        dependency: bool,
    ) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;

        Ok(
            match dependency {
                true => Request2::depdendency(&path, content),
                false => Request2::new(&path, content),
            }
        )
    }

    pub fn method(&self) -> Result<Method> {
        let first_line = self.first_line()?;
        let split: Vec<&str> = first_line.splitn(2, ' ').collect();
        let method_string = split[0];

        Method::from_str(method_string)
            .map_err(|_| FhttpError::new(ErrorKind::RequestParseException(format!("Couldn't parse method '{}'", method_string))))
    }

    pub fn url(&self) -> Result<&str> {
        let first_line = self.first_line()?;
        let mut split: Vec<&str> = first_line.splitn(2, ' ').collect();

        split.pop()
            .ok_or(FhttpError::new(ErrorKind::RequestParseException("Malformed url line".into())))
    }

    pub fn headers(&self) -> Result<HeaderMap> {
        let lines = self.text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with('#'))
            .skip(1)
            .collect::<Vec<&str>>();

        let mut ret = HeaderMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }

            let split: Vec<&str> = line.splitn(2, ':').collect();
            let key = HeaderName::from_str(split[0].trim())
                .expect("couldn't create HeaderName");
            let value_text = split[1].trim();
            let value = HeaderValue::from_str(value_text).unwrap();
            ret.insert(key, value);
        }

        if self.gql_file() {
            ret.entry("content-type")
                .or_insert(HeaderValue::from_static("application/json"));
        }

        Ok(ret)
    }

    pub fn body(&self) -> Result<Cow<str>> {
        if self.gql_file() {
            Ok(Cow::Owned(self._gql_body()?))
        } else {
            Ok(Cow::Borrowed(self._body()?))
        }
    }

    pub fn response_handler(&self) -> Result<Option<Box<dyn ResponseHandler>>> {
        lazy_static! {
            static ref RE_RESPONSE_HANDLER: Regex = Regex::new(r"(?sm)>\s*\{%(.*)%}").unwrap();
        };

        if let Some(captures) = RE_RESPONSE_HANDLER.captures(&self.text) {
            if let Some(group) = captures.get(1) {
                let group = group.as_str().trim();
                let parts: Vec<&str> = group.splitn(2, ' ').collect();
                let kind = parts[0];
                let content = parts[1];

                match kind {
                    "json" => Ok(Some(Box::new(JsonPathResponseHandler::new(content)))),
                    unknown => Err(FhttpError::new(ErrorKind::RequestParseException(format!("Unknown response handler '{}'", unknown))))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn dependencies(&self) -> Vec<PathBuf> {
        let mut ret = vec![];
        for capture in RE_REQUEST.captures_iter(&self.text) {
            let group = capture.get(1).unwrap().as_str();
            let path = self.get_dependency_path(group);
            ret.push(path);
        }
        ret
    }

    fn _body(&self) -> Result<&str> {
        let mut body_start = None;
        let mut body_end = None;
        let mut chars: usize = 0;
        let mut last_char = None;

        for (index, chr) in self.text.chars().enumerate() {
            if body_start.is_none() && chr == '\n' && last_char == Some('\n') {
                body_start = Some(chars + 1);
            } else if body_end.is_none() && chr == '%' && &self.text[(index - 4)..index] == "\n> {" {
                body_end = Some(index - 4);
                break;
            }

            last_char = Some(chr);
            chars += 1;
        }

        let body_start = body_start.unwrap();
        let body_end = body_end.unwrap_or(chars);

        Ok(&self.text[body_start..body_end])
    }

    fn _gql_body(&self) -> Result<String> {
        let body = self._body()?;
        let parts: Vec<&str> = body.split("\n\n").collect();

        let (query, variables) = match parts.len() {
            1 => (parts[0], None),
            2 => (parts[0], Some(parse_variables(parts[1])?)),
            _ => return Err(FhttpError::new(ErrorKind::RequestParseException("GraphQL requests can only have 1 or 2 body parts".into()))),
        };

        let query = Value::String(query.to_owned());

        let mut map = Map::new();
        map.insert("query".into(), query);
        map.insert("variables".into(), variables.unwrap_or(Value::Object(Map::new())));
        let body = Value::Object(map);

        Ok(serde_json::to_string(&body).unwrap())
    }

    fn first_line(&self) -> Result<&str> {
        self.text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with("#"))
            .nth(0)
            .ok_or(FhttpError::new(ErrorKind::RequestParseException("Could not find first line".into())))
    }

    fn gql_file(&self) -> bool {
        let filename = self.source_path.file_name().unwrap().to_str().unwrap();

        filename.ends_with(".gql.http") || filename.ends_with(".graphql.http")
    }

    pub fn get_dependency_path(
        &self,
        path: &str
    ) -> PathBuf {
        get_dependency_path(
            &self.source_path,
            path
        )
    }
}

impl PartialEq for Request2 {
    fn eq(
        &self,
        other: &Self
    ) -> bool {
        self.source_path == other.source_path
    }
}

fn parse_variables(text: &str) -> Result<Value> {
    serde_json::from_str::<Value>(&text)
        .map_err(|_| FhttpError::new(ErrorKind::RequestParseException("Error parsing variables".into())))
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::*;

    #[test]
    fn method() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.method()?, Method::POST);

        Ok(())
    }

    #[test]
    fn method_no_first_line() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            # POST http://localhost:8080
        "##));

        match req.method() {
            Err(FhttpError { kind: ErrorKind::RequestParseException(ref msg) }) => {
                assert_eq!(msg, "Could not find first line");
            },
            _ => panic!("Expected error!")
        }

        Ok(())
    }

    #[test]
    fn url() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
        "##));

        assert_eq!(req.url()?, "http://localhost:8080");

        Ok(())
    }

    #[test]
    fn headers() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            # comment
            POST http://localhost:8080
            # comment
            content-type: application/json; charset=UTF-8
            accept: application/json

            not-a-header: not-a-header-value
        "##));

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json; charset=UTF-8").unwrap());
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json").unwrap());
        assert_eq!(req.headers()?, expected_headers);

        Ok(())
    }

    #[test]
    fn body() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            POST http://localhost:8080

            this is the body

            this as well

            > {%
                json $
            %}
        "##));

        assert_eq!(
            req.body()?,
            indoc!(r##"
                this is the body

                this as well
            "##)
        );

        Ok(())
    }

    #[test]
    fn response_handler() -> Result<()> {
        let req = Request2::new(std::env::current_dir()?, indoc!(r##"
            POST http://localhost:8080

            this is the body

            > {%
                json $
            %}
        "##));

        assert!(req.response_handler()?.is_some());

        Ok(())
    }
}

#[cfg(test)]
mod gql {
    use std::fs;

    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use reqwest::Method;
    use serde_json::json;

    use indoc::indoc;

    use super::*;

    #[test]
    fn parse_gql_with_query_variables_response_handler() -> Result<()> {
        let source_path = std::env::current_dir()?.join("foo.gql.http");
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

        let result = Request2::new(&source_path, input);

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {
                "var": "entity-id"
            }
        });
        let body = serde_json::from_str::<Value>(&result.body()?)?;

        assert_eq!(result.method()?, Method::POST);
        assert_eq!(result.url()?, "http://server:8080/graphql");
        assert_eq!(result.headers()?, headers);
        assert_eq!(body, expected_body);
        assert_eq!(result.source_path, source_path);
        assert_eq!(result.dependency, false);
        assert!(result.response_handler()?.is_some());

        Ok(())
    }

    #[test]
    fn parse_gql_with_query_variables() -> Result<()> {
        let source_path = std::env::current_dir()?.join("foo.gql.http");
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

        let result = Request2::new(&source_path, input);

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {
                "var": "entity-id"
            }
        });
        let body = serde_json::from_str::<Value>(&result.body()?)?;

        assert_eq!(result.method()?, Method::POST);
        assert_eq!(result.url()?, "http://server:8080/graphql");
        assert_eq!(result.headers()?, headers);
        assert_eq!(body, expected_body);
        assert_eq!(result.source_path, source_path);
        assert_eq!(result.dependency, false);
        assert!(result.response_handler()?.is_none());

        Ok(())
    }

    #[test]
    fn parse_gql_with_query_response_handler() -> Result<()> {
        let source_path = std::env::current_dir()?.join("foo.gql.http");
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

        let result = Request2::new(&source_path, input);

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}\n",
            "variables": {}
        });
        let body = serde_json::from_str::<Value>(&result.body()?)?;

        assert_eq!(result.method()?, Method::POST);
        assert_eq!(result.url()?, "http://server:8080/graphql");
        assert_eq!(result.headers()?, headers);
        assert_eq!(body, expected_body);
        assert_eq!(result.source_path, source_path);
        assert_eq!(result.dependency, false);
        assert!(result.response_handler()?.is_some());

        Ok(())
    }

    #[test]
    fn parse_gql_with_query() -> Result<()> {
        let source_path = std::env::current_dir()?.join("foo.gql.http");
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

        let result = Request2::new(
            &source_path,
            input
        );

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}\n",
            "variables": {}
        });

        let body = serde_json::from_str::<Value>(&result.body()?)?;

        assert_eq!(result.method()?, Method::POST);
        assert_eq!(result.url()?, "http://server:8080/graphql");
        assert_eq!(result.headers()?, headers);
        assert_eq!(body, expected_body);
        assert_eq!(result.source_path, source_path);
        assert_eq!(result.dependency, false);
        assert!(result.response_handler()?.is_none());

        Ok(())
    }

    #[test]
    fn parse_should_parse_gql_based_on_filename() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/gql");
        let http_extension = root.join("request.http");
        let gql_http_extension = root.join("request.gql.http");

        let http_extension_result = Request2::new(
            &http_extension,
            fs::read_to_string(&http_extension)?
        );

        let gql_http_extension_result = Request2::new(
            &gql_http_extension,
            fs::read_to_string(&gql_http_extension)?,
        );

        assert!(&http_extension_result.body()?.starts_with("query"));

        assert!(serde_json::from_str::<Value>(&gql_http_extension_result.body()?).is_ok());
        match serde_json::from_str::<Value>(&gql_http_extension_result.body()?)? {
            Value::Object(map) => {
                assert!(map.contains_key("query"));
                assert!(map.contains_key("variables"));
            },
            _ => panic!("expected a Value::Object!")
        }

        Ok(())
    }

    #[test]
    fn parse_qgl_should_set_contenttype_if_not_given() -> Result<()> {
        let dummy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/dummy.gql.http");
        let json = HeaderValue::from_str("application/json").unwrap();
        let xml = HeaderValue::from_str("application/xml").unwrap();

        let req = Request2::new(
            &dummy_path,
            indoc!(r##"
            POST http://graphql

            query {
                foo
            }
            "##)
        );
        assert!(req.headers()?.contains_key(&HeaderName::from_str("content-type").unwrap()));
        assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&json));

        let req = Request2::new(
            &dummy_path,
            indoc!(r##"
            POST http://graphql
            Content-type: application/xml

            query {
                foo
            }
            "##),
        );
        assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&xml));

        Ok(())
    }
}

#[cfg(test)]
mod dependencies {
    use super::*;

    #[test]
    fn should_find_dependencies() -> Result<()> {
        let source_path = std::env::current_dir()?;
        let input = format!(r##"GET http://${{request("resources/test/requests/nested_dependencies/1.http")}}:8080
Authorization: Bearer ${{request("./../fhttp/resources/test/requests/nested_dependencies/2.http")}}

${{request("{}")}}
"##,
            source_path.join("resources/test/requests/nested_dependencies/3.http").to_str().unwrap()
        );

        let req = Request2::new(&source_path, input);
        let dependencies = req.dependencies();

        assert_eq!(
            dependencies,
            vec![
                source_path.join("resources/test/requests/nested_dependencies/1.http"),
                source_path.join("resources/test/requests/nested_dependencies/2.http"),
                source_path.join("resources/test/requests/nested_dependencies/3.http"),
            ]
        );

        Ok(())
    }
}
