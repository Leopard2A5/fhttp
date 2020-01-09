use crate::Request;
use crate::FhttpError;
use core::iter::Iterator;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;
use std::env;
use reqwest::header::HeaderValue;
use std::fs;
use crate::errors::ErrorKind;
use crate::Result;

lazy_static!{
    static ref RE_REQUEST: Regex = Regex::new(r#"(?m)\$\{request\("([^"]+)"\)}"#).unwrap();
}

pub struct RequestPreprocessor {
    requests: Vec<Request>,
    response_data: HashMap<PathBuf, String>
}

impl RequestPreprocessor {
    pub fn new(requests: Vec<Request>) -> Result<Self> {
        let mut preprocessor_stack = vec![];
        let mut requests_with_dependencies = vec![];

        for req in requests {
            preprocess_request(req, &mut requests_with_dependencies, &mut preprocessor_stack)?;
        }


        Ok(
            RequestPreprocessor {
                requests: requests_with_dependencies.into_iter().collect(),
                response_data: HashMap::new()
            }
        )
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn notify_response(
        &mut self,
        path: &Path,
        response: &str
    ) {
        let path = fs::canonicalize(&path).unwrap();

        self.response_data.insert(
            path,
            response.to_owned()
        );
    }
}

fn replace_env_vars(req: &mut Request) -> Result<()> {
    req.url = eval(&req.url)?;

    for (_, value) in req.headers.iter_mut() {
        *value = HeaderValue::from_str(&eval(&value.to_str()?)?)?;
    }

    req.body = eval(&req.body)?;

    Ok(())
}

fn eval(text: &str) -> Result<String> {
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
        let value = env::var(key)
            .map_err(|_| FhttpError::new(ErrorKind::MissingEnvVar(key.into())))?;
        buffer.replace_range(range, &value);
    }

    Ok(buffer)
}

fn preprocess_request(
    mut req: Request,
    mut list: &mut Vec<Request>,
    mut preprocessor_stack: &mut Vec<PathBuf>
) -> Result<()> {
    if list.contains(&req) {
        return Ok(());
    }
    if preprocessor_stack.contains(&req.source_path) {
        panic!("cyclic dependency detected!");
    }
    preprocessor_stack.push(req.source_path.clone());

    replace_env_vars(&mut req)?;

    for dep in get_dependencies(&req)? {
        preprocess_request(dep, &mut list, &mut preprocessor_stack)?;
    }

    preprocessor_stack.pop();
    list.push(req);

    Ok(())
}

fn get_dependencies(req: &Request) -> Result<Vec<Request>> {
    let mut ret: Vec<Request> = vec![];

    let url_deps = get_dependencies_from_str(&req.source_path, &req.url)?;
    let body_deps = get_dependencies_from_str(&req.source_path, &req.body)?;

    let mut header_deps = vec![];
    for header in req.headers.values() {
        let text = header.to_str()?;
        let deps = get_dependencies_from_str(&req.source_path, &text)?;
        header_deps.extend(deps);
    }

    ret.extend(url_deps);
    ret.extend(header_deps);
    ret.extend(body_deps);

    Ok(ret)
}

fn get_dependencies_from_str(
    origin_path: &Path,
    text: &str
) -> Result<Vec<Request>> {
    let mut ret = vec![];
    for capture in RE_REQUEST.captures_iter(&text) {
        let group = capture.get(1).unwrap().as_str();
        let path = get_dependency_path(&origin_path, group);
        ret.push(Request::parse_dependency(
            std::fs::read_to_string(&path)?,
            &path
        ));
    }

    Ok(ret)
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
            let mut req = self.requests.remove(0);
            replace_dependency_values(&mut req, &self.response_data).unwrap();
            Some(req)
        } else {
            None
        }
    }
}

fn replace_dependency_values(
    req: &mut Request,
    response_data: &HashMap<PathBuf, String>
) -> Result<()> {
    req.url = replace_dependency_values_in_str(&req.source_path, &req.url, &response_data);

    for (_, value) in req.headers.iter_mut() {
        let replaced = &replace_dependency_values_in_str(&req.source_path, value.to_str()?, &response_data);
        let new_value = HeaderValue::from_str(replaced)?;
        *value = new_value;
    }

    req.body = replace_dependency_values_in_str(&req.source_path, &req.body, &response_data);

    Ok(())
}

fn replace_dependency_values_in_str(
    source_path: &Path,
    text: &str,
    response_data: &HashMap<PathBuf, String>
) -> String {
    let reversed_captures = RE_REQUEST.captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

    let mut ret = text.to_owned();
    for capture in reversed_captures {
        let whole_match = capture.get(0).unwrap();
        let range = whole_match.start()..whole_match.end();

        let group = capture.get(1).unwrap();
        let path = get_dependency_path(&source_path, &group.as_str());

        let replacement = response_data.get(&path).unwrap();
        ret.replace_range(range, &replacement);
    }

    ret
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
        assert_eq!(eval(input).unwrap(), "XfooXbarX");
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

        replace_env_vars(&mut req).unwrap();
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

        replace_env_vars(&mut req).unwrap();
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

        replace_env_vars(&mut req).unwrap();
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

        let mut preprocessor = RequestPreprocessor::new(vec![init_request])
            .unwrap();
        for i in 2..=5 {
            let path = root.join(format!("{}.http", i));
            preprocessor.notify_response(&path, &format!("{}", i));
        }

        let coll = preprocessor.into_iter()
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

        let mut preprocessor = RequestPreprocessor::new(vec![req1, req2])
            .unwrap();
        preprocessor.notify_response(&dep_path, "");
        let coll = preprocessor
            .map(|it| it.source_path)
            .collect::<Vec<_>>();
        assert_eq!(&coll, &[dep_path, path1, path2]);
    }

    #[test]
    #[should_panic]
    fn should_panic_on_cyclic_dependency() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/cyclic_dependencies");
        let path1 = root.join("1.http");
        let req1 = Request::parse(
            fs::read_to_string(&path1).unwrap(),
            &path1
        );

        RequestPreprocessor::new(vec![req1]).unwrap();
    }

    #[test]
    #[should_panic]
    fn should_panic_on_missing_dependency_response() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("4.http");

        let init_request = Request::parse(
            fs::read_to_string(&init_path).unwrap(),
            &init_path
        );

        let mut preprocessor = RequestPreprocessor::new(vec![init_request])
            .unwrap();
        preprocessor.next();
        preprocessor.next();
    }

    #[test]
    fn should_replace_dependencies_on_next_calls() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("4.http");
        let dep_path = root.join("5.http");

        let init_request = Request::parse(
            fs::read_to_string(&init_path).unwrap(),
            &init_path
        );

        let mut preprocessor = RequestPreprocessor::new(vec![init_request])
            .unwrap();
        preprocessor.next();
        preprocessor.notify_response(&dep_path, "dependency");
        let result = preprocessor.next().unwrap();
        assert_eq!(result.url, "dependency");
    }
}

#[cfg(test)]
mod replace_dependency_values_in_str {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn should_replace_dependencies_in_str() {
        let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let path1 = source_path.join("1.http");
        let path2 = source_path.join("2.http");

        let mut dependency_values = HashMap::new();
        dependency_values.insert(path1.clone(), "dep1".to_owned());
        dependency_values.insert(path2.clone(), "dep2".to_owned());

        let input = r#"X${request("1.http")}-${request("2.http")}X"#.to_owned();
        let result = replace_dependency_values_in_str(&source_path, &input, &dependency_values);

        assert_eq!(result, "Xdep1-dep2X");
    }
}

#[cfg(test)]
mod replace_dependency_values {
    use super::*;
    use std::path::PathBuf;
    use std::str::FromStr;
    use crate::Request;
    use reqwest::header::{HeaderMap, HeaderName};

    #[test]
    fn should_replace_dependencies() {
        let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let path1 = source_path.join("1.http");

        let mut headers = HeaderMap::new();
        let header_name = HeaderName::from_str("key").unwrap();
        headers.insert(
            header_name.clone(),
            HeaderValue::from_str(r#"${request("1.http")}"#).unwrap()
        );
        let mut request = Request {
            method: Default::default(),
            url: r#"${request("1.http")}"#.to_string(),
            headers: headers.clone(),
            body: r#"${request("1.http")}"#.to_string(),
            source_path: source_path.clone(),
            response_handler: None,
            dependency: false
        };

        let mut dependency_values = HashMap::new();
        dependency_values.insert(path1.clone(), "dep1".to_owned());

        replace_dependency_values(&mut request, &dependency_values).unwrap();
        assert_eq!(request.url, "dep1");
        assert_eq!(request.body, "dep1");
        assert_eq!(request.headers.get(&header_name), Some(&HeaderValue::from_str("dep1").unwrap()));
    }
}
