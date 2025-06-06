use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use pest::iterators::Pair;
use pest::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::parsers::normal_parser::{RequestParser, Rule};
use crate::parsers::{fileupload_regex, Request};
use crate::path_utils::RelativePath;
use crate::postprocessing::response_handler::ResponseHandler;
use crate::request::body::{Body, MultipartPart};

pub fn parse_str<P: AsRef<Path>, T: AsRef<str>>(path: P, source: T) -> Result<Request> {
    let path = path.as_ref();
    let file = RequestParser::parse(Rule::file, source.as_ref())
        .with_context(|| format!("failed to parse file {}", path.to_str().unwrap()))?
        .next()
        .unwrap(); // get and unwrap the `file` rule; never fails

    let mut method = Method::GET;
    let mut url = String::new();
    let mut headers = HeaderMap::new();
    let mut body = String::new();
    let mut response_handler: Option<ResponseHandler> = None;

    for element in file.into_inner() {
        match element.as_rule() {
            Rule::first_line => parse_first_line(element, &mut method, &mut url)?,
            Rule::header_line => parse_header_line(&mut headers, element)?,
            Rule::body => body.push_str(element.as_str().trim()),
            Rule::response_handler_json => {
                parse_json_response_handler(&mut response_handler, element)
            }
            Rule::response_handler_deno => {
                parse_deno_response_handler(&mut response_handler, element)
            }
            Rule::response_handler_rhai => {
                parse_rhai_response_handler(&mut response_handler, element)
            }
            _ => (),
        }
    }

    Ok(Request {
        method,
        url,
        headers,
        body: plain_body_or_files(path, body)?,
        response_handler,
    })
}

fn parse_first_line(element: Pair<Rule>, method: &mut Method, url: &mut String) -> Result<()> {
    for field in element.into_inner() {
        match field.as_rule() {
            Rule::method => {
                *method = Method::from_str(field.as_str())
                    .with_context(|| format!("invalid method '{}'", field.as_str()))?
            }
            Rule::url => url.push_str(field.as_str()),
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn parse_header_line(headers: &mut HeaderMap, element: Pair<Rule>) -> Result<()> {
    let mut name = String::new();
    let mut value = String::new();

    for part in element.into_inner() {
        match part.as_rule() {
            Rule::header_name => name.push_str(part.as_str()),
            Rule::header_value => value.push_str(part.as_str()),
            _ => unreachable!(),
        }
    }

    headers.insert(
        HeaderName::from_str(&name).with_context(|| format!("invalid header name: '{}'", &name))?,
        HeaderValue::from_str(&value)
            .with_context(|| format!("invalid header value: '{}'", &value))?,
    );

    Ok(())
}

fn parse_json_response_handler(
    response_handler: &mut Option<ResponseHandler>,
    element: Pair<Rule>,
) {
    element.into_inner().for_each(|exp| match exp.as_rule() {
        Rule::response_handler_exp => {
            *response_handler = Some(ResponseHandler::Json {
                json_path: exp.as_str().trim().to_owned(),
            });
        }
        _ => unreachable!(),
    });
}

fn parse_deno_response_handler(
    response_handler: &mut Option<ResponseHandler>,
    element: Pair<Rule>,
) {
    element.into_inner().for_each(|exp| match exp.as_rule() {
        Rule::response_handler_exp => {
            *response_handler = Some(ResponseHandler::Deno {
                program: exp.as_str().trim().to_owned(),
            });
        }
        _ => unreachable!(),
    });
}

fn parse_rhai_response_handler(
    response_handler: &mut Option<ResponseHandler>,
    element: Pair<Rule>,
) {
    element.into_inner().for_each(|exp| match exp.as_rule() {
        Rule::response_handler_exp => {
            *response_handler = Some(ResponseHandler::Rhai {
                program: exp.as_str().trim().to_owned(),
            });
        }
        _ => unreachable!(),
    });
}

fn plain_body_or_files(source_path: &Path, body: String) -> Result<Body> {
    let captures = fileupload_regex().captures_iter(&body);

    if captures.count() == 0 {
        Ok(Body::Plain(body))
    } else {
        // TODO can we reuse captures?
        let files = fileupload_regex()
            .captures_iter(&body)
            .map(|capture| {
                let name = capture.get(1).unwrap().as_str().to_owned();
                let path = capture.get(2).unwrap().as_str();
                let file_path = source_path.get_dependency_path(path)?;
                Ok(MultipartPart::File {
                    name,
                    file_path,
                    mime_str: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Body::Multipart(files))
    }
}

#[cfg(test)]
mod parse_normal_requests {
    use std::env::current_dir;

    use crate::test_utils::root;
    use indoc::indoc;

    use super::*;

    #[test]
    fn should_parse_simple_delete() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
        );

        Ok(())
    }

    #[test]
    fn should_parse_headers() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml
            com.header.name: com.header.value
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .add_header("com.header.name", "com.header.value")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_body() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            body
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo").body("body")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_json_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            > {%
                json $.data
            %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo").response_handler_json("$.data")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_rhai_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            > {%
                rhai
                (2 + 2).to_string()
            %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo").response_handler_rhai("(2 + 2).to_string()")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_deno_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            > {%
                deno setResult('ok');
            %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .response_handler_deno("setResult('ok');")
        );

        Ok(())
    }

    #[test]
    fn should_parse_with_body_and_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            body
            body

            > {%
                json $.data
            %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .body("body\nbody")
                .response_handler_json("$.data")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_headers_and_body() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo
            foo: bar



            body
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("foo", "bar")
                .body("body")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_headers_and_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo
            foo: bar



            > {% json foo %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .add_header("foo", "bar")
                .response_handler_json("foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_more_space_between_body_and_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo

            body



            > {% json foo %}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
                .body("body")
                .response_handler_json("foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_just_the_first_line() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            DELETE http://localhost:9000/foo



        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("DELETE", "http://localhost:9000/foo")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_headers() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml



        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_body() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            body


        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .body("body")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            > {% json handler %}



        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .response_handler_json("handler")
        );

        Ok(())
    }

    #[test]
    fn should_tolerate_trailing_newlines_with_body_and_response_handler() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            content-type: application/json; charset=UTF-8
            accept: application/xml

            body

            > {% json handler %}



        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("content-type", "application/json; charset=UTF-8")
                .add_header("accept", "application/xml")
                .body("body")
                .response_handler_json("handler")
        );

        Ok(())
    }

    #[test]
    fn should_allow_commenting_out_headers() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo
            # content-type: application/json; charset=UTF-8
            accept: application/xml
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo")
                .add_header("accept", "application/xml")
        );

        Ok(())
    }

    #[test]
    fn should_parse_file_bodies() -> Result<()> {
        let result = parse_str(
            current_dir().unwrap(),
            indoc!(
                r##"
            GET http://localhost:9000/foo

            ${file("partname", "../resources/it/profiles.json")}
            ${file("file", "../resources/it/profiles2.json")}
        "##
            ),
        )?;

        assert_eq!(
            result,
            Request::basic("GET", "http://localhost:9000/foo").multipart(&[
                MultipartPart::File {
                    name: "partname".to_string(),
                    file_path: root().join("resources/it/profiles.json"),
                    mime_str: None,
                },
                MultipartPart::File {
                    name: "file".to_string(),
                    file_path: root().join("resources/it/profiles2.json"),
                    mime_str: None,
                },
            ])
        );

        Ok(())
    }
}
