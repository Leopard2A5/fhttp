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
        let mut requests_with_dependencies = vec![];

        for req in requests {
            preprocess_request(req, &mut requests_with_dependencies);
        }

        RequestPreprocessor {
            requests: requests_with_dependencies.into_iter().collect(),
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

fn replace_env_vars(req: &mut Request) {
    req.url = eval(&req.url);

    for (_, value) in req.headers.iter_mut() {
        *value = HeaderValue::from_str(&eval(&value.to_str().unwrap())).unwrap();
    }

    req.body = eval(&req.body);
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

fn preprocess_request(
    mut req: Request,
    mut list: &mut Vec<Request>
) {
    if list.contains(&req) {
        return;
    }

    replace_env_vars(&mut req);

    for dep in get_dependencies(&req) {
        preprocess_request(dep, &mut list);
    }

    list.push(req);
}

fn get_dependencies(req: &Request) -> Vec<Request> {
    let mut ret = vec![];

    let url_deps = get_dependencies_from_str(&req.source_path, &req.url);
    let header_deps = req.headers.values().flat_map(|header_value| {
        let text = header_value.to_str().unwrap();
        get_dependencies_from_str(&req.source_path, &text)
    }).collect::<Vec<_>>();
    let body_deps = get_dependencies_from_str(&req.source_path, &req.body);

    ret.extend(url_deps);
    ret.extend(header_deps);
    ret.extend(body_deps);

    ret
}

fn get_dependencies_from_str(
    origin_path: &Path,
    text: &str
) -> Vec<Request> {
    lazy_static!{
        static ref RE_REQUEST: Regex = Regex::new(r#"(?m)\$\{request\("([^"]+)"\)}"#).unwrap();
    };

    let mut ret = vec![];
    for capture in RE_REQUEST.captures_iter(&text) {
        let group = capture.get(1).unwrap().as_str();
        let path = get_dependency_path(&origin_path, group);
        ret.push(Request::parse(
            std::fs::read_to_string(&path).unwrap(),
            &path
        ));
    }

    ret
}

fn get_dependency_path(
    origin_path: &Path,
    path: &str
) -> PathBuf {
    let path = Path::new(path);
    let ret = if path.is_absolute() {
        path.to_path_buf()
    } else if origin_path.is_dir() {
        origin_path.join(path).to_path_buf()
    } else {
        origin_path.parent().unwrap().join(path).to_path_buf()
    };

    std::fs::canonicalize(ret).unwrap()
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
        let mut req = Request::parse(
            "GET http://${env(SERVER)}:${env(PORT)}/".into(),
            &env::current_dir().unwrap()
        );

        replace_env_vars(&mut req);
        assert_eq!(req.url, "http://localhost:8080/");
    }

    #[test]
    fn should_replace_in_headers() {
        env::set_var("E1", "e1");
        env::set_var("E2", "e2");
        env::set_var("E3", "e3");
        let mut req = Request::parse(
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

        replace_env_vars(&mut req);
        assert_eq!(req.headers, headers);
    }

    #[test]
    fn should_replace_in_body() {
        env::set_var("E1", "e1");
        env::set_var("E2", "e2");
        let mut req = Request::parse(
            indoc!("
                GET http://localhost/

                E1=${env(E1)} + E2=${env(E2)}
            ").into(),
            &env::current_dir().unwrap()
        );

        replace_env_vars(&mut req);
        assert_eq!(req.body, "E1=e1 + E2=e2");
    }
}

#[cfg(test)]
mod dependencies {
    use super::*;
    use crate::Request;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn should_resolve_nested_dependencies() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("1.http");

        let init_request = Request::parse(
            fs::read_to_string(&init_path).unwrap(),
            &init_path
        );

        let mut coll = Vec::new();
        preprocess_request(init_request, &mut coll);

        let coll = coll.into_iter()
            .map(|it| it.source_path)
            .collect::<Vec<_>>();

        let foo = (1..=5).into_iter()
            .rev()
            .map(|i| root.join(format!("{}.http", i)))
            .collect::<Vec<_>>();
        assert_eq!(&coll, &foo);
    }

    #[test]
    fn should_not_resolve_duplicate_dependencies() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/duplicate_dependencies");
        let path1 = root.join("1.http");
        let path2 = root.join("2.http");
        let dep_path = root.join("dependency.http");

        let req1 = Request::parse(
            fs::read_to_string(&path1).unwrap(),
            &path1
        );
        let req2 = Request::parse(
            fs::read_to_string(&path2).unwrap(),
            &path2
        );

        let preprocessor = RequestPreprocessor::new(vec![req1, req2]);
        let coll = preprocessor.into_iter()
            .map(|it| it.source_path)
            .collect::<Vec<_>>();
        assert_eq!(&coll, &[dep_path, path1, path2]);
    }
}
