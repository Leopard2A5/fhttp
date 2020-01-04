use crate::Request;
use core::iter::Iterator;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;
use std::env;
use reqwest::header::HeaderValue;

pub struct RequestPreprocessor {
    requests: Vec<Request>,
    response_data: HashMap<PathBuf, String>
}

impl RequestPreprocessor {
    pub fn new(requests: Vec<Request>) -> Self {
        RequestPreprocessor {
            requests,
            response_data: HashMap::new()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn notify_response(
        &mut self,
        path: &Path,
        response: &str
    ) {
        self.response_data.insert(
            path.to_path_buf(),
            response.to_owned()
        );
    }
}

fn replace_env_vars(mut req: Request) -> Request {
    req.url = eval(&req.url);

    for (_, value) in req.headers.iter_mut() {
        *value = HeaderValue::from_str(&eval(&value.to_str().unwrap())).unwrap();
    }

    req
}

fn eval(text: &str) -> String {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{env\(([^}]+)\)}").unwrap();
    };

    let mut buffer = text.to_owned();
    let reversed_captures = RE_ENV.captures_iter(text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    for capture in reversed_captures {
        let group = capture.get(0).unwrap();
        let key = capture.get(1).unwrap().as_str();
        let range = group.start()..group.end();
        buffer.replace_range(range, &env::var(key).unwrap());
    }

    buffer
}

impl Iterator for RequestPreprocessor {
    type Item = Request;

    fn next(&mut self) -> Option<Self::Item> {
        if self.requests.len() > 0 {
            Some(self.requests.remove(0))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod eval {
    use super::*;
    use std::env;

    #[test]
    fn eval_should_replace_with_env_vars() {
        env::set_var("FOO", "foo");
        env::set_var("BAR", "bar");
        let input = "X${env(FOO)}X${env(BAR)}X";
        assert_eq!(eval(input), "XfooXbarX");
    }
}

#[cfg(test)]
mod replace_env_vars {
    use super::*;
    use crate::Request;
    use std::env;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use std::str::FromStr;
    use indoc::indoc;

    #[test]
    fn should_replace_in_url() {
        env::set_var("SERVER", "localhost");
        env::set_var("PORT", "8080");
        let req = Request::parse(
            "GET http://${env(SERVER)}:${env(PORT)}/".into(),
            &env::current_dir().unwrap()
        );

        let result = replace_env_vars(req);
        assert_eq!(result.url, "http://localhost:8080/");
    }

    #[test]
    fn should_replace_in_headers() {
        env::set_var("E1", "e1");
        env::set_var("E2", "e2");
        env::set_var("E3", "e3");
        let req = Request::parse(
            indoc!("
                GET http://localhost/
                H1: ${env(E1)}
                H23: ${env(E2)}, ${env(E3)}
            ").into(),
            &env::current_dir().unwrap()
        );

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_str("H1").unwrap(), HeaderValue::from_str("e1").unwrap());
        headers.insert(HeaderName::from_str("H23").unwrap(), HeaderValue::from_str("e2, e3").unwrap());

        let result = replace_env_vars(req);
        assert_eq!(result.headers, headers);
    }

}
