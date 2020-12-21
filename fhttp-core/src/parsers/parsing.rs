use std::path::Path;
use std::str::FromStr;

use pest::iterators::Pair;
use pest::Parser;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::errors::{FhttpError, Result};
use crate::parsers::normal_parser::{RequestParser, Rule};
use crate::parsers::ParsedRequest;
use crate::path_utils::RelativePath;
use crate::request::body::{Body, File};
use crate::response_handler::ResponseHandler;

pub fn parse_str<T: AsRef<str>>(
    path: &Path,
    source: T
) -> Result<ParsedRequest> {
    let file = RequestParser::parse(Rule::file, source.as_ref())
        .map_err(|e| {
            FhttpError::new(format!("failed to parse file {} {}", path.to_str().unwrap(), e.to_string()))
        })?
        .next().unwrap(); // get and unwrap the `file` rule; never fails

    let mut method = Method::GET;
    let mut url = String::new();
    let mut headers = HeaderMap::new();
    let mut body = String::new();
    let mut response_handler: Option<ResponseHandler> = None;

    for element in file.into_inner() {
        match element.as_rule() {
            Rule::first_line => parse_first_line(element, &mut method, &mut url)?,
            Rule::header_line => parse_header_line(&mut headers, element)?,
            Rule::body => body.push_str(&element.as_str().trim()),
            Rule::response_handler_json => parse_json_response_handler(&mut response_handler, element),
            _ => ()
        }
    }

    Ok(
        ParsedRequest {
            method,
            url,
            headers,
            body: plain_body_or_files(&path, body),
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
            Rule::method => *method = Method::from_str(&field.as_str())
                .map_err(|_| FhttpError::new(format!("invalid method '{}'", &field.as_str())))?,
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

fn plain_body_or_files(
    source_path: &Path,
    body: String,
) -> Body {
    lazy_static! {
        static ref RE_FILE: Regex = Regex::new(r##"(?m)\$\{\s*file\s*\(\s*"([^}]+)"\s*,\s*"([^}]+)"\s*\)\s*\}"##).unwrap();
    };

    let captures = RE_FILE.captures_iter(&body)
        .collect::<Vec<_>>();

    if captures.len() == 0 {
        Body::Plain(body)
    } else {
        let files = RE_FILE.captures_iter(&body)
            .map(|capture| {
                let name = capture.get(1).unwrap().as_str().to_owned();
                let path = capture.get(2).unwrap().as_str();
                let path = source_path.get_dependency_path(path);
                File { name, path }
            })
            .collect::<Vec<_>>();
        Body::Files(files)
    }
}

#[cfg(test)]
mod parse_normal_requests {
    use std::env::current_dir;

    use indoc::indoc;

    use super::*;

    #[test]
    fn should_parse_simple_delete() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo
        "##))?;

        assert_eq!(result, ParsedRequest::basic("DELETE", "http://localhost:9000/foo"));

        Ok(())
    }

    #[test]
    fn should_parse_headers() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_body() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo

            body
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .body("body")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo

            > {%
                json $.data
            %}
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .response_handler_json("$.data")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_body_and_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo

            body
            body

            > {%
                json $.data
            %}
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .body("body\nbody")
                .response_handler_json("$.data")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_headers_and_body() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo
            foo: bar



            body
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .add_header("foo", "bar")
                .body("body")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_headers_and_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo
            foo: bar



            > {% json foo %}
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .add_header("foo", "bar")
                .response_handler_json("foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_body_and_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo

            body



            > {% json foo %}
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("DELETE", "http://localhost:9000/foo")
                .body("body")
                .response_handler_json("foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_just_the_first_line() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            DELETE http://localhost:9000/foo



        "##))?;

        assert_eq!(result, ParsedRequest::basic("DELETE", "http://localhost:9000/foo"));

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_headers() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml



        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_body() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            body


        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .body("body")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            > {% json handler %}



        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .response_handler_json("handler")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_body_and_response_handler() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            body

            > {% json handler %}



        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .body("body")
                .response_handler_json("handler")
        );

        Ok(())
    }

    #[test]
    fn should_allow_commenting_out_headers() -> Result<()> {
        let result = parse_str(&current_dir().unwrap(), indoc!(r##"
            GET http://localhost:9000/foo
            # content-type: application/json; charset=UTF-8
            accept: application/xml
        "##))?;

        assert_eq!(
            result,
            ParsedRequest::basic("GET", "http://localhost:9000/foo")
                .add_header("accept", "application/xml")
        );

        Ok(())
    }
}
