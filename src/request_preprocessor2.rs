use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use regex::{Captures, Regex};

use crate::{Config, Profile};
use crate::random_numbers::replace_random_ints;
use crate::request2::RE_REQUEST;
use crate::request2::Request2;
use crate::Result;
use crate::uuids::replace_uuids;

#[derive(Debug)]
pub struct RequestPreprocessor2 {
    profile: Profile,
    config: Config,
    requests: Vec<Request2>,
    response_data: HashMap<PathBuf, String>,
}

impl RequestPreprocessor2 {

    pub fn new(
        profile: Profile,
        requests: Vec<Request2>,
        config: Config,
    ) -> Result<Self> {
        let mut preprocessor_stack = vec![];
        let mut requests_with_dependencies = vec![];

        for req in requests {
            preprocess_request(
                &profile,
                req,
                &mut requests_with_dependencies,
                &mut preprocessor_stack,
                &config
            )?;
        }

        Ok(
            RequestPreprocessor2 {
                profile,
                config,
                requests: requests_with_dependencies,
                response_data: HashMap::new(),
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

    fn replace_variables(
        &self,
        req: Request2
    ) -> Result<Request2> {
        let text = self.replace_env_vars(req.text)?;
        let text = replace_uuids(text);
        let text = replace_random_ints(text)?;
        let text = self.replace_dependency_values(text, &req.source_path)?;

        Ok(
            Request2 {
                text,
                ..req
            }
        )
    }

    fn replace_env_vars(
        &self,
        text: String
    ) -> Result<String> {
        lazy_static! {
            static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{env\(([^}]+)\)}").unwrap();
        };

        let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&text)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        if reversed_captures.is_empty() {
            Ok(text)
        } else {
            let mut buffer = text.clone();
            for capture in reversed_captures {
                let group = capture.get(0).unwrap();
                let key = capture.get(1).unwrap().as_str();
                let range = group.start()..group.end();
                let value = self.profile.get(key, self.config.prompt_missing_env_vars)?;

                buffer.replace_range(range, &value);
            }
            Ok(buffer)
        }
    }

    fn replace_dependency_values(
        &self,
        text: String,
        source_path: &Path
    ) -> Result<String> {
        let reversed_captures = RE_REQUEST.captures_iter(&text)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        if reversed_captures.is_empty() {
            Ok(text)
        } else {
            let mut ret = text.clone();
            for capture in reversed_captures {
                let whole_match = capture.get(0).unwrap();
                let range = whole_match.start()..whole_match.end();

                let group = capture.get(1).unwrap();
                let path = get_dependency_path(&source_path, &group.as_str());

                let replacement = self.response_data.get(&path).unwrap();
                ret.replace_range(range, &replacement);
            }

            Ok(ret)
        }
    }
}

impl Iterator for RequestPreprocessor2 {
    type Item = Result<Request2>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.requests.len() > 0 {
            let req = self.requests.remove(0);
            let req = self.replace_variables(req);
            Some(req)
        } else {
            None
        }
    }
}

pub fn get_dependency_path(
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

fn preprocess_request(
    profile: &Profile,
    req: Request2,
    mut list: &mut Vec<Request2>,
    mut preprocessor_stack: &mut Vec<PathBuf>,
    config: &Config
) -> Result<()> {
    if list.contains(&req) {
        return Ok(());
    }
    if preprocessor_stack.contains(&req.source_path) {
        panic!("cyclic dependency detected!");
    }
    preprocessor_stack.push(req.source_path.clone());

    for dep in req.dependencies() {
        let dep = Request2::from_file(&dep, true)?;
        preprocess_request(&profile, dep, &mut list, &mut preprocessor_stack, &config)?;
    }

    preprocessor_stack.pop();
    list.push(req);

    Ok(())
}

#[cfg(test)]
mod env_vars {
    use std::env;

    use indoc::indoc;

    use super::*;

    #[test]
    fn should_replace_env_vars() -> Result<()> {
        env::set_var("SERVER", "server");
        env::set_var("TOKEN", "token");
        env::set_var("BODY", "body");

        let req = Request2::new(
            env::current_dir()?,
            indoc!(r##"
                GET http://${env(SERVER)}
                Authorization: ${env(TOKEN)}

                X${env(BODY)}X
            "##)
        );
        let mut processor = RequestPreprocessor2::new(
            Profile::new(),
            vec![req],
            Config::default()
        )?;
        let next = processor.next().unwrap()?;

        assert_eq!(
            &next.text,
            indoc!(r##"
                GET http://server
                Authorization: token

                XbodyX
            "##)
        );

        Ok(())
    }
}

#[cfg(test)]
mod uuids {
    use std::env;

    use indoc::indoc;

    use super::*;

    #[test]
    fn should_replace_uuids() -> Result<()> {
        lazy_static! {
            static ref REGEX: Regex = Regex::new(r"X[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}X").unwrap();
        };

        let req = Request2::new(
            env::current_dir()?,
            indoc!(r##"
                GET http://X${uuid()}X
            "##)
        );
        let mut processor = RequestPreprocessor2::new(
            Profile::new(),
            vec![req],
            Config::default()
        )?;
        let next = processor.next().unwrap()?;

        assert!(REGEX.is_match(&next.text));

        Ok(())
    }
}

#[cfg(test)]
mod dependencies {
    use std::fs;
    use std::path::PathBuf;

    use crate::request2::Request2;

    use super::*;

    #[test]
    fn should_resolve_nested_dependencies() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("1.http");

        let init_request = Request2::from_file(&init_path, false)?;

        let mut preprocessor = RequestPreprocessor2::new(
            Profile::new(),
            vec![init_request],
            Config::default()
        )?;

        for i in 2..=5 {
            let path = root.join(format!("{}.http", i));
            preprocessor.notify_response(&path, &format!("{}", i));
        }

        let coll = preprocessor.into_iter()
            .map(|it| it.unwrap().source_path)
            .collect::<Vec<_>>();

        let foo = (1..=5).into_iter()
            .rev()
            .map(|i| root.join(format!("{}.http", i)))
            .collect::<Vec<_>>();
        assert_eq!(&coll, &foo);

        Ok(())
    }

    #[test]
    fn should_not_resolve_duplicate_dependencies() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/duplicate_dependencies");
        let path1 = root.join("1.http");
        let path2 = root.join("2.http");
        let dep_path = root.join("dependency.http");

        let req1 = Request2::from_file(&path1, false)?;
        let req2 = Request2::from_file(&path2, false)?;

        let mut preprocessor = RequestPreprocessor2::new(
            Profile::new(),
            vec![req1, req2],
            Config::default()
        )?;
        preprocessor.notify_response(&dep_path, "");
        let coll = preprocessor
            .map(|it| it.unwrap().source_path)
            .collect::<Vec<_>>();
        assert_eq!(&coll, &[dep_path, path1, path2]);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn should_panic_on_cyclic_dependency() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/cyclic_dependencies");
        let path1 = root.join("1.http");
        let req1 = Request2::from_file(&path1, false).unwrap();

        RequestPreprocessor2::new(
            Profile::new(),
            vec![req1],
            Config::default()
        ).unwrap();
    }

    #[test]
    #[should_panic]
    fn should_panic_on_missing_dependency_response() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("4.http");
        let init_request = Request2::from_file(&init_path, false).unwrap();

        let mut preprocessor = RequestPreprocessor2::new(
            Profile::new(),
            vec![init_request],
            Config::default()
        ).unwrap();

        preprocessor.next();
        preprocessor.next();
    }

    #[test]
    fn should_replace_dependencies_on_next_calls() -> Result<()> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("4.http");
        let dep_path = root.join("5.http");

        let init_request = Request2::from_file(&init_path, false)?;

        let mut preprocessor = RequestPreprocessor2::new(
            Profile::new(),
            vec![init_request],
            Config::default()
        )?;

        preprocessor.next();
        preprocessor.notify_response(&dep_path, "dependency");
        let result = preprocessor.next().unwrap().unwrap();
        assert_eq!(result.url()?, "dependency");

        Ok(())
    }
}
