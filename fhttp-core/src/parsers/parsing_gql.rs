use std::str::FromStr;

use pest::iterators::Pair;
use pest::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use serde_json::map::Map;
use serde_json::Value;

use crate::errors::{FhttpError, Result};
use crate::parsers::gql_parser::{RequestParser, Rule};
use crate::parsers::Request;
use crate::request_def::body::Body;
use crate::response_handler::ResponseHandler;

pub fn parse_gql_str<T: AsRef<str>>(source: T) -> Result<Request> {
    let file = RequestParser::parse(Rule::file, source.as_ref())
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `file` rule; never fails

    let mut method = Method::GET;
    let mut url = String::new();
    let mut headers = HeaderMap::new();
    let mut query = String::new();
    let mut response_handler: Option<ResponseHandler> = None;
    let mut variables: Option<String> = None;

    for element in file.into_inner() {
        match element.as_rule() {
            Rule::first_line => parse_first_line(element, &mut method, &mut url)?,
            Rule::header_line => parse_header_line(&mut headers, element)?,
            Rule::query => query.push_str(element.as_str().trim()),
            Rule::variables => variables = Some(element.as_str().trim().to_owned()),
            Rule::response_handler_json => parse_json_response_handler(&mut response_handler, element),
            Rule::response_handler_deno => parse_deno_response_handler(&mut response_handler, element),
            _ => ()
        }
    }

    let variables = match variables {
        None => Value::Object(Map::new()),
        Some(ref variables) => serde_json::from_str(variables)
            .map_err(|_| FhttpError::new("Error parsing variables section, seems to be invalid JSON?"))?,
    };

    disallow_file_uploads(&query)?;

    let mut map = Map::new();
    map.insert("query".into(), Value::String(query));
    map.insert("variables".into(), variables);

    let body = Value::Object(map);
    let body = serde_json::to_string(&body).unwrap();
    let body = Body::Plain(body);

    Ok(
        Request {
            method,
            url,
            headers: ensure_content_type_json(headers),
            body,
            response_handler,
        }
    )
}

fn parse_first_line(
    element: Pair<Rule>,
    method: &mut Method,
    url: &mut String,
) -> Result<()> {
    for field in element.into_inner() {
        match field.as_rule() {
            Rule::method => *method = Method::from_str(field.as_str())
                .map_err(|_| FhttpError::new(format!("invalid method '{}'", field.as_str())))?,
            Rule::url => url.push_str(field.as_str()),
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn parse_header_line(
    headers: &mut HeaderMap,
    element: Pair<Rule>,
) -> Result<()> {
    let mut name = String::new();
    let mut value = String::new();

    for part in element.into_inner() {
        match part.as_rule() {
            Rule::header_name => name.push_str(part.as_str()),
            Rule::header_value => value.push_str(part.as_str()),
            _ => unreachable!()
        }
    }

    headers.insert(
        HeaderName::from_str(&name).map_err(|_| FhttpError::new(format!("invalid header name: '{}'", &name)))?,
        HeaderValue::from_str(&value).map_err(|_| FhttpError::new(format!("invalid header value: '{}'", &value)))?
    );

    Ok(())
}

fn parse_json_response_handler(
    response_handler: &mut Option<ResponseHandler>,
    element: Pair<Rule>,
) {
    for exp in element.into_inner() {
        match exp.as_rule() {
            Rule::response_handler_exp => {
                *response_handler = Some(
                    ResponseHandler::Json {
                        json_path: exp.as_str().trim().to_owned()
                    }
                );
                return;
            },
            _ => unreachable!()
        }
    }
}

fn parse_deno_response_handler(
    response_handler: &mut Option<ResponseHandler>,
    element: Pair<Rule>,
) {
    for exp in element.into_inner() {
        match exp.as_rule() {
            Rule::response_handler_exp => {
                *response_handler = Some(
                    ResponseHandler::Deno {
                        program: exp.as_str().trim().to_owned()
                    }
                );
                return;
            },
            _ => unreachable!()
        }
    }
}

fn ensure_content_type_json(mut map: HeaderMap) -> HeaderMap {
    map.entry("content-type").or_insert(HeaderValue::from_static("application/json"));

    map
}

fn disallow_file_uploads(body: &str) -> Result<()> {
    use crate::parsers::file_upload_regex;

    let captures = file_upload_regex::RE_FILE.captures_iter(body);

    match captures.count() {
        0 => Ok(()),
        _ => Err(FhttpError::new("file uploads are not allowed in graphql requests"))
    }
}

#[cfg(test)]
mod parse_gql_requests {
    use indoc::indoc;
    use serde_json::json;

    use super::*;

    #[test]
    fn should_parse_headers_and_query() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml
            com.header.name: com.header.value

            query
        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .add_header("com.header.name", "com.header.value")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
        );

        Ok(())
    }

    #[test]
    fn should_allow_overriding_content_type() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/xml

            query
        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/xml")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
        );

        Ok(())
    }

    #[test]
    fn should_parse_query_and_response_handler() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            DELETE http://localhost:9000/foo

            query
            query

            > {%
                json $.data
            %}
        "##))?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .gql_body(json!({
                    "query": "query\nquery",
                    "variables": {}
                }))
                .response_handler_json("$.data")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_deno_response_handler() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            DELETE http://localhost:9000/foo

            query
            query

            > {%
                deno
                if (status === 200) {
                    setResult('ok');
                } else {
                    setResult('not ok');
                }
            %}
        "##))?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .gql_body(json!({
                    "query": "query\nquery",
                    "variables": {}
                }))
                .response_handler_deno(r#"if (status === 200) {
        setResult('ok');
    } else {
        setResult('not ok');
    }"#
                )
        );

        Ok(())
    }

    #[test]
    fn should_parse_query_and_variables() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo

            query

            {
                "foo": "bar"
            }
        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .gql_body(json!({
                    "query": "query",
                    "variables": {
                        "foo": "bar"
                    }
                }))
        );

        Ok(())
    }

    #[test]
    fn should_parse_query_variables_and_response_handler() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            DELETE http://localhost:9000/foo

            query
            query

            { "foo": "bar" }

            > {%
                json $.data
            %}
        "##))?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .body("query\nquery")
                .response_handler_json("$.data")
                .gql_body(json!({
                    "query": "query\nquery",
                    "variables": {
                        "foo": "bar"
                    }
                }))
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_headers_and_query() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            DELETE http://localhost:9000/foo
            foo: bar



            query
        "##))?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .add_header("foo", "bar")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_query_and_response_handler() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            DELETE http://localhost:9000/foo

            query



            > {% json foo %}
        "##))?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
                .response_handler_json("foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_query() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            query


        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_query_and_response_handler() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            query

            > {% json handler %}



        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
                .response_handler_json("handler")
        );

        Ok(())
    }

    #[test]
    fn should_allow_commenting_out_headers() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo
            # accept: application/xml

            query
        "##))?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json")
                .gql_body(json!({
                    "query": "query",
                    "variables": {}
                }))
        );

        Ok(())
    }

    #[test]
    fn should_not_allow_using_file_uploads_in_gql_files() -> Result<()> {
        let result = parse_gql_str(indoc!(r##"
            GET http://localhost:9000/foo

            ${file("partname", "../resources/it/profiles.json")}
            ${file("file", "../resources/it/profiles2.json")}
        "##));

        assert_eq!(
            result,
            Err(FhttpError::new("file uploads are not allowed in graphql requests"))
        );

        Ok(())
    }
}
