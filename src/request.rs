use std::path::{Path, PathBuf};
use std::str::{FromStr, Lines};

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;

use crate::response_handler::{JsonPathResponseHandler, ResponseHandler};

struct Request {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: String,
    pub source_path: PathBuf,
    pub response_handler: Option<Box<dyn ResponseHandler>>
}

impl Request {

    pub fn parse(
        input: String,
        path: &Path
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
            source_path: path.to_path_buf(),
            response_handler
        }
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
        let value_text = split[1];
        let values: Vec<HeaderValue> = value_text
            .split(',')
            .map(|it| it.trim())
            .map(|it| HeaderValue::from_str(it).expect("couldn't create HeaderValue"))
            .collect();
        for value in values {
            headers.insert(key.clone(), value);
        }
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
    let captures = RE_RESPONSE_HANDLER.captures(&text).unwrap();

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
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::*;

    #[test]
    fn should_parse_request() {
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
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/json").unwrap());
        expected_headers.insert(HeaderName::from_str("accept").unwrap(), HeaderValue::from_str("application/xml").unwrap());

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

}
