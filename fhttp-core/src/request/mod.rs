use std::cell::RefCell;
use std::path::{Path, PathBuf};

use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::Method;

use crate::errors::Result;
use crate::parsers::{parse_gql_str, parse_str, ParsedRequest};
use crate::path_utils::{RelativePath, canonicalize};
use crate::request::body::Body;
use crate::request::has_body::HasBody;
use crate::response_handler::ResponseHandler;
use crate::file_includes::load_file_recursively;

pub mod variable_support;
pub mod body;
pub mod has_body;

lazy_static!{
    pub static ref RE_REQUEST: Regex = Regex::new(r#"(?m)\$\{request\("([^"]+)"\)}"#).unwrap();
}

// #[derive(Debug, Eq)]
pub struct Request {
    pub source_path: PathBuf,
    pub text: String,
    pub dependency: bool,
    parsed_request: RefCell<Option<ParsedRequest>>,
}

impl Request {

    pub fn from_file(
        path: &Path,
        dependency: bool,
    ) -> Result<Self> {
        let path = canonicalize(&path)?;
        let content = load_file_recursively(&path)?;

        Request::_new(&path, content, dependency)
    }

    #[cfg(test)]
    fn new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Result<Self> {
        Request::_new(path, text, false)
    }

    fn _new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T,
        dependency: bool
    ) -> Result<Self> {
        let ret = Request {
            source_path: path.into(),
            text: text.into(),
            dependency,
            parsed_request: RefCell::new(None),
        };

        Ok(ret)
    }

    pub fn method(&self) -> Result<Method> {
        self.parsed_request(|pr| pr.method.clone())
    }

    pub fn url(&self) -> Result<String> {
        self.parsed_request(|pr| pr.url.clone())
    }

    pub fn headers(&self) -> Result<HeaderMap> {
        self.parsed_request(|pr| pr.headers.clone())
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

    pub fn gql_file(&self) -> bool {
        let filename = self.source_path.file_name().unwrap().to_str().unwrap();

        filename.ends_with(".gql.http") || filename.ends_with(".graphql.http")
    }

    pub fn response_handler(&self) -> Result<Option<ResponseHandler>> {
        self.parsed_request(|pr| pr.response_handler.clone())
    }

    fn parsed_request<F, R>(
        &self,
        func: F,
    ) -> Result<R>
    where F: Fn(&ParsedRequest) -> R {
        if RefCell::borrow(&self.parsed_request).is_none() {
            self.parse()?;
        }

        match RefCell::borrow(&self.parsed_request).as_ref() {
            None => unreachable!(),
            Some(pr) => Ok(func(pr)),
        }
    }

    fn parse(&self) -> Result<()> {
        self.parsed_request.replace(
            Some(
                match self.gql_file() {
                    true => parse_gql_str(&self.text)?,
                    false => parse_str(&self.source_path, &self.text)?,
                }
            )
        );

        Ok(())
    }

    fn _body(&self) -> Result<&str> {
        let mut body_start = None;
        let mut body_end = None;
        let mut text_index: usize = 0;
        let mut last_char = None;

        for (index, chr) in self.text.chars().enumerate() {
            if body_start.is_none() && chr == '\n' && last_char == Some('\n') {
                body_start = Some(text_index + 1);
            } else if body_end.is_none() && chr == '%' && &self.text[(index - 4)..index] == "\n> {" {
                body_end = Some(index - 4);
                break;
            }

            last_char = Some(chr);
            text_index += 1;
        }

        match body_start {
            Some(start) => {
                let end = body_end.unwrap_or(text_index);
                if start < end {
                    Ok(&self.text[start..body_end.unwrap_or(text_index)])
                } else {
                    Ok("")
                }
            },
            None => Ok(""),
        }
    }
}

impl HasBody for Request {
    fn body(&self) -> Result<Body> {
        self.parsed_request(|pr| pr.body.clone())
    }
}

impl AsRef<Path> for Request {
    fn as_ref(&self) -> &Path {
        &self.source_path
    }
}

impl PartialEq for Request {
    fn eq(
        &self,
        other: &Self
    ) -> bool {
        self.source_path == other.source_path
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use indoc::indoc;
    use reqwest::header::{HeaderName, HeaderValue};

    use crate::errors::Result;
    use crate::request::body::Body;
    use crate::request::has_body::HasBody;
    use crate::test_utils::errmsg;

    use super::*;

    #[test]
    fn method() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            # comment
            POST http://localhost:8080
        "##))?;

        assert_eq!(req.method()?, Method::POST);

        Ok(())
    }

    #[test]
    fn method_no_first_line() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            # comment
            # POST http://localhost:8080
        "##))?;

        assert!(errmsg(req.method()).contains("expected method"));

        Ok(())
    }

    #[test]
    fn url() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            # comment
            POST http://localhost:8080
        "##))?;

        assert_eq!(req.url()?, "http://localhost:8080");

        Ok(())
    }

    #[test]
    fn headers() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            # comment
            POST http://localhost:8080
            # comment
            content-type: application/json; charset=UTF-8
            accept: application/json

            not-a-header: not-a-header-value
        "##))?;

        let mut expected_headers = HeaderMap::new();
        expected_headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json; charset=UTF-8").unwrap());
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json").unwrap());
        assert_eq!(req.headers()?, expected_headers);

        Ok(())
    }

    #[test]
    fn body() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            POST http://localhost:8080

            this is the body
            this as well

            > {%
                json $
            %}
        "##))?;

        assert_eq!(
            req.body()?,
            Body::plain(indoc!(r##"
                this is the body
                this as well"##
            ))
        );

        Ok(())
    }

    #[test]
    fn no_body_should_return_empty_string() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            POST http://localhost:8080
        "##))?;

        assert_eq!(req.body()?, Body::plain(""));

        Ok(())
    }

    #[test]
    fn no_body_with_response_handler_should_return_empty_string() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            POST http://localhost:8080

            > {%
                json $
            %}
        "##))?;

        assert_eq!(req.body()?, Body::plain(""));

        Ok(())
    }
}

#[cfg(test)]
mod fileupload {
    use indoc::indoc;

    use crate::errors::Result;
    use crate::request::body::{Body, File};
    use crate::request::has_body::HasBody;
    use crate::test_utils::root;

    use super::*;

    #[test]
    fn test() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            POST http://localhost:8080

            ${file("partname", "../resources/it/profiles.json")}
            ${file(
                "file",
                "../resources/it/profiles2.json"
            )}
        "##))?;

        assert_eq!(
            req.body()?,
            Body::Files(vec![
                File {
                    name: String::from("partname"),
                    path: root().join("resources/it/profiles.json")
                },
                File {
                    name: String::from("file"),
                    path: root().join("resources/it/profiles2.json")
                }
            ])
        );

        Ok(())
    }

}

#[cfg(test)]
mod gql {
    use std::fs;
    use std::str::FromStr;

    use indoc::indoc;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use reqwest::Method;
    use serde_json::json;
    use serde_json::value::Value;

    use crate::errors::Result;
    use crate::request::body::Body;
    use crate::request::has_body::HasBody;
    use crate::test_utils::root;

    use super::*;

    #[test]
    fn parse_gql_with_query_variables_response_handler() -> Result<()> {
        let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
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

        let result = Request::new(&source_path, input)?;

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {
                "var": "entity-id"
            }
        });
        let body = match result.body()? {
            Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
            _ => panic!("aaaaah!")
        };

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
        let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
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

        let result = Request::new(&source_path, input)?;

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {
                "var": "entity-id"
            }
        });
        let body = match result.body()? {
            Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
            _ => panic!("aaaaah!"),
        };

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
        let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
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

        let result = Request::new(&source_path, input)?;

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {}
        });
        let body = match result.body()? {
            Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
            _ => panic!("aaaaah!"),
        };

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
        let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
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

        let result = Request::new(
            &source_path,
            input
        )?;

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
        headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());

        let expected_body = json!({
            "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
            "variables": {}
        });

        let body = match result.body()? {
            Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
            _ => panic!("aaaaah!"),
        };

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
        let root = root()
            .join("resources/test/requests/gql");
        let http_extension = root.join("request.http");
        let gql_http_extension = root.join("request.gql.http");

        let http_extension_result = Request::new(
            &http_extension,
            fs::read_to_string(&http_extension).unwrap()
        )?;

        let gql_http_extension_result = Request::new(
            &gql_http_extension,
            fs::read_to_string(&gql_http_extension).unwrap(),
        )?;

        match http_extension_result.body()? {
            Body::Plain(body) => assert!(&body.starts_with("query")),
            _ => panic!("aaaah!"),
        };

        let json_body = match gql_http_extension_result.body()? {
            Body::Plain(body) => serde_json::from_str::<Value>(&body),
            _ => panic!("aaaaah!"),
        };

        assert!(json_body.is_ok());
        match json_body.unwrap() {
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

        let req = Request::new(
            &dummy_path,
            indoc!(r##"
            POST http://graphql

            query {
                foo
            }
            "##)
        )?;
        assert!(req.headers()?.contains_key(&HeaderName::from_str("content-type").unwrap()));
        assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&json));

        let req = Request::new(
            &dummy_path,
            indoc!(r##"
            POST http://graphql
            Content-type: application/xml

            query {
                foo
            }
            "##),
        )?;
        assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&xml));

        Ok(())
    }
}

#[cfg(test)]
mod dependencies {
    use crate::errors::Result;
    use crate::test_utils::root;

    use super::*;

    #[test]
    fn should_find_dependencies() -> Result<()> {
        let source_path = root();
        let input = format!(r##"GET http://${{request("resources/test/requests/nested_dependencies/1.http")}}:8080
Authorization: Bearer ${{request("./../fhttp/resources/test/requests/nested_dependencies/2.http")}}

${{request("{}")}}
"##,
            source_path.join("resources/test/requests/nested_dependencies/3.http").to_str().unwrap()
        );

        let req = Request::new(&source_path, input)?;
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
