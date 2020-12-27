#[cfg(test)]
use std::path::PathBuf;
use std::path::Path;

use regex::Regex;

use crate::errors::Result;
use crate::file_includes::load_file_recursively;
use crate::parsers::{parse_gql_str, parse_str};
use crate::path_utils::{canonicalize, RelativePath, CanonicalizedPathBuf};
use crate::request_def::request_wrapper::RequestWrapper;

pub mod variable_support;
pub mod body;
pub mod request_wrapper;

lazy_static!{
    pub static ref RE_REQUEST: Regex = Regex::new(r#"(?m)\$\{request\("([^"]+)"\)}"#).unwrap();
}

// #[derive(Debug, Eq)]
pub struct RequestDef {
    pub source_path: CanonicalizedPathBuf,
    pub text: String,
    pub dependency: bool,
}

impl RequestDef {

    pub fn from_file<P: AsRef<Path>>(
        path: P,
        dependency: bool,
    ) -> Result<Self> {
        let path = canonicalize(path.as_ref())?;
        let content = load_file_recursively(&path)?;

        RequestDef::_new(path, content, dependency)
    }

    #[cfg(test)]
    fn new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Result<Self> {
        let path = canonicalize(&path.into())?;
        RequestDef::_new(path, text, false)
    }

    fn _new<T: Into<String>>(
        path: CanonicalizedPathBuf,
        text: T,
        dependency: bool
    ) -> Result<Self> {
        let ret = RequestDef {
            source_path: path,
            text: text.into(),
            dependency,
        };

        Ok(ret)
    }

    pub fn dependencies(&self) -> Result<Vec<CanonicalizedPathBuf>> {
        RE_REQUEST.captures_iter(&self.text)
            .map(|capture| capture.get(1).unwrap().as_str())
            .map(|path| self.get_dependency_path(path))
            .collect()
    }

    pub fn parse(self) -> Result<RequestWrapper> {
        let request = match self.gql_file() {
            true => parse_gql_str(&self.text)?,
            false => parse_str(&self.source_path, &self.text)?,
        };

        Ok(
            RequestWrapper {
                source_path: self.source_path,
                request,
            }
        )
    }

    fn gql_file(&self) -> bool {
        let filename = self.source_path.file_name();

        filename.ends_with(".gql.http") || filename.ends_with(".graphql.http")
    }
}

impl AsRef<Path> for RequestDef {
    fn as_ref(&self) -> &Path {
        &self.source_path.as_ref()
    }
}

impl PartialEq for RequestDef {
    fn eq(
        &self,
        other: &Self
    ) -> bool {
        self.source_path == other.source_path
    }
}

// #[cfg(test)]
// mod gql {
//     use std::fs;
//     use std::str::FromStr;
//
//     use indoc::indoc;
//     use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
//     use reqwest::Method;
//     use serde_json::json;
//     use serde_json::value::Value;
//
//     use crate::errors::Result;
//     use crate::request_def::body::Body;
//     use crate::test_utils::root;
//
//     use super::*;
//
//     #[test]
//     fn parse_gql_with_query_variables_response_handler() -> Result<()> {
//         let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
//         let input = indoc!(r##"
//             POST http://server:8080/graphql
//             Authorization: Bearer token
//
//             query($var: String!) {
//                 entity(id: $var, foo: "bar") {
//                     field1
//                     field2
//                 }
//             }
//
//             {
//                 "var": "entity-id"
//             }
//
//             > {%
//                 json $
//             %}
//         "##).to_owned();
//
//         let result = RequestDef::new(&source_path, input)?;
//
//         let mut headers = HeaderMap::new();
//         headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
//         headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());
//
//         let expected_body = json!({
//             "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
//             "variables": {
//                 "var": "entity-id"
//             }
//         });
//         let body = match result.body()? {
//             Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
//             _ => panic!("aaaaah!")
//         };
//
//         assert_eq!(result.method()?, Method::POST);
//         assert_eq!(result.url()?, "http://server:8080/graphql");
//         assert_eq!(result.headers()?, headers);
//         assert_eq!(body, expected_body);
//         assert_eq!(result.source_path, source_path);
//         assert_eq!(result.dependency, false);
//         assert!(result.response_handler()?.is_some());
//
//         Ok(())
//     }
//
//     #[test]
//     fn parse_gql_with_query_variables() -> Result<()> {
//         let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
//         let input = indoc!(r##"
//             POST http://server:8080/graphql
//             Authorization: Bearer token
//
//             query($var: String!) {
//                 entity(id: $var, foo: "bar") {
//                     field1
//                     field2
//                 }
//             }
//
//             {
//                 "var": "entity-id"
//             }
//         "##).to_owned();
//
//         let result = RequestDef::new(&source_path, input)?;
//
//         let mut headers = HeaderMap::new();
//         headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
//         headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());
//
//         let expected_body = json!({
//             "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
//             "variables": {
//                 "var": "entity-id"
//             }
//         });
//         let body = match result.body()? {
//             Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
//             _ => panic!("aaaaah!"),
//         };
//
//         assert_eq!(result.method()?, Method::POST);
//         assert_eq!(result.url()?, "http://server:8080/graphql");
//         assert_eq!(result.headers()?, headers);
//         assert_eq!(body, expected_body);
//         assert_eq!(result.source_path, source_path);
//         assert_eq!(result.dependency, false);
//         assert!(result.response_handler()?.is_none());
//
//         Ok(())
//     }
//
//     #[test]
//     fn parse_gql_with_query_response_handler() -> Result<()> {
//         let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
//         let input = indoc!(r##"
//             POST http://server:8080/graphql
//             Authorization: Bearer token
//
//             query($var: String!) {
//                 entity(id: $var, foo: "bar") {
//                     field1
//                     field2
//                 }
//             }
//
//             > {%
//                 json $
//             %}
//         "##).to_owned();
//
//         let result = RequestDef::new(&source_path, input)?;
//
//         let mut headers = HeaderMap::new();
//         headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
//         headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());
//
//         let expected_body = json!({
//             "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
//             "variables": {}
//         });
//         let body = match result.body()? {
//             Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
//             _ => panic!("aaaaah!"),
//         };
//
//         assert_eq!(result.method()?, Method::POST);
//         assert_eq!(result.url()?, "http://server:8080/graphql");
//         assert_eq!(result.headers()?, headers);
//         assert_eq!(body, expected_body);
//         assert_eq!(result.source_path, source_path);
//         assert_eq!(result.dependency, false);
//         assert!(result.response_handler()?.is_some());
//
//         Ok(())
//     }
//
//     #[test]
//     fn parse_gql_with_query() -> Result<()> {
//         let source_path = std::env::current_dir().unwrap().join("foo.gql.http");
//         let input = indoc!(r##"
//             POST http://server:8080/graphql
//             Authorization: Bearer token
//
//             query($var: String!) {
//                 entity(id: $var, foo: "bar") {
//                     field1
//                     field2
//                 }
//             }
//         "##).to_owned();
//
//         let result = RequestDef::new(
//             &source_path,
//             input
//         )?;
//
//         let mut headers = HeaderMap::new();
//         headers.insert(HeaderName::from_str("Authorization").unwrap(), HeaderValue::from_str("Bearer token").unwrap());
//         headers.insert(HeaderName::from_str("content-type").unwrap(), HeaderValue::from_str("application/json").unwrap());
//
//         let expected_body = json!({
//             "query": "query($var: String!) {\n    entity(id: $var, foo: \"bar\") {\n        field1\n        field2\n    }\n}",
//             "variables": {}
//         });
//
//         let body = match result.body()? {
//             Body::Plain(body) => serde_json::from_str::<Value>(&body).unwrap(),
//             _ => panic!("aaaaah!"),
//         };
//
//         assert_eq!(result.method()?, Method::POST);
//         assert_eq!(result.url()?, "http://server:8080/graphql");
//         assert_eq!(result.headers()?, headers);
//         assert_eq!(body, expected_body);
//         assert_eq!(result.source_path, source_path);
//         assert_eq!(result.dependency, false);
//         assert!(result.response_handler()?.is_none());
//
//         Ok(())
//     }
//
//     #[test]
//     fn parse_should_parse_gql_based_on_filename() -> Result<()> {
//         let root = root()
//             .join("resources/test/requests/gql");
//         let http_extension = root.join("request.http");
//         let gql_http_extension = root.join("request.gql.http");
//
//         let http_extension_result = RequestDef::new(
//             &http_extension,
//             fs::read_to_string(&http_extension).unwrap()
//         )?;
//
//         let gql_http_extension_result = RequestDef::new(
//             &gql_http_extension,
//             fs::read_to_string(&gql_http_extension).unwrap(),
//         )?;
//
//         match http_extension_result.body()? {
//             Body::Plain(body) => assert!(&body.starts_with("query")),
//             _ => panic!("aaaah!"),
//         };
//
//         let json_body = match gql_http_extension_result.body()? {
//             Body::Plain(body) => serde_json::from_str::<Value>(&body),
//             _ => panic!("aaaaah!"),
//         };
//
//         assert!(json_body.is_ok());
//         match json_body.unwrap() {
//             Value::Object(map) => {
//                 assert!(map.contains_key("query"));
//                 assert!(map.contains_key("variables"));
//             },
//             _ => panic!("expected a Value::Object!")
//         }
//
//         Ok(())
//     }
//
//     #[test]
//     fn parse_qgl_should_set_contenttype_if_not_given() -> Result<()> {
//         let dummy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//             .join("resources/test/requests/dummy.gql.http");
//         let json = HeaderValue::from_str("application/json").unwrap();
//         let xml = HeaderValue::from_str("application/xml").unwrap();
//
//         let req = RequestDef::new(
//             &dummy_path,
//             indoc!(r##"
//             POST http://graphql
//
//             query {
//                 foo
//             }
//             "##)
//         )?;
//         assert!(req.headers()?.contains_key(&HeaderName::from_str("content-type").unwrap()));
//         assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&json));
//
//         let req = RequestDef::new(
//             &dummy_path,
//             indoc!(r##"
//             POST http://graphql
//             Content-type: application/xml
//
//             query {
//                 foo
//             }
//             "##),
//         )?;
//         assert_eq!(req.headers()?.get(&HeaderName::from_str("content-type").unwrap()), Some(&xml));
//
//         Ok(())
//     }
// }
//
// #[cfg(test)]
// mod dependencies {
//     use crate::errors::Result;
//     use crate::test_utils::root;
//
//     use super::*;
//
//     #[test]
//     fn should_find_dependencies() -> Result<()> {
//         let source_path = root();
//         let input = format!(r##"GET http://${{request("resources/test/requests/nested_dependencies/1.http")}}:8080
// Authorization: Bearer ${{request("./../fhttp/resources/test/requests/nested_dependencies/2.http")}}
//
// ${{request("{}")}}
// "##,
//             source_path.join("resources/test/requests/nested_dependencies/3.http").to_str().unwrap()
//         );
//
//         let req = RequestDef::new(&source_path, input)?;
//         let dependencies = req.dependencies();
//
//         assert_eq!(
//             dependencies,
//             vec![
//                 source_path.join("resources/test/requests/nested_dependencies/1.http"),
//                 source_path.join("resources/test/requests/nested_dependencies/2.http"),
//                 source_path.join("resources/test/requests/nested_dependencies/3.http"),
//             ]
//         );
//
//         Ok(())
//     }
// }
