use std::fs;
use std::path::Path;

use fhttp_core::{Config, ResponseStore};
use fhttp_core::{Request, RE_REQUEST};
use fhttp_core::VariableSupport;
use fhttp_core::Result;
use fhttp_core::path_utils;
use fhttp_core::execution_order::plan_request_order;
use fhttp_core::{Profile, Resolve};
use crate::random_numbers::replace_random_ints;
use crate::uuids::replace_uuids;
use std::ops::Range;

#[derive(Debug)]
pub struct Requestpreprocessor {
    profile: Profile,
    config: Config,
    requests: Vec<Request>,
    response_data: ResponseStore,
}

impl Requestpreprocessor {

    pub fn new(
        profile: Profile,
        requests: Vec<Request>,
        config: Config,
    ) -> Result<Self> {
        let requests_in_order = plan_request_order(requests, &profile, &config)?;

        Ok(
            Requestpreprocessor {
                profile,
                config,
                requests: requests_in_order,
                response_data: ResponseStore::new(),
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

        self.response_data.store(path, response);
    }

    fn replace_variables(
        &self,
        req: Request
    ) -> Result<Request> {
        let text = self.replace_env_vars(&req)?;
        let text = replace_uuids(text);
        let text = replace_random_ints(text)?;
        let text = self.replace_dependency_values(text, &req.source_path)?;

        Ok(
            Request {
                text,
                ..req
            }
        )
    }

    fn replace_env_vars(
        &self,
        req: &Request
    ) -> Result<String> {
        let reversed_captures: Vec<(&str, Range<usize>)> = req.get_env_vars();

        if reversed_captures.is_empty() {
            Ok(req.text.clone())
        } else {
            let mut buffer = req.text.clone();
            for (key, range) in reversed_captures {
                let value = match self.profile.get(key, self.config.prompt_missing_env_vars)? {
                    Resolve::StringValue(value) => value,
                    Resolve::RequestLookup(path) => {
                        let path = path_utils::get_dependency_path(self.profile.source_path(), path.to_str().unwrap());
                        self.response_data.get(&path)
                    },
                };

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
                let path = path_utils::get_dependency_path(&source_path, &group.as_str());

                let replacement = self.response_data.get(&path);
                ret.replace_range(range, &replacement);
            }

            Ok(ret)
        }
    }
}

impl Iterator for Requestpreprocessor {
    type Item = Result<Request>;

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

        let req = Request::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://${env(SERVER)}
                Authorization: ${env(TOKEN)}

                X${env(BODY)}X
            "##)
        );
        let mut processor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
        use regex::Regex;
        lazy_static! {
            static ref REGEX: Regex = Regex::new(r"X[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}X").unwrap();
        };

        let req = Request::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://X${uuid()}X
            "##)
        );
        let mut processor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
    use std::env;
    use std::path::PathBuf;

    use fhttp_core::Request;
    use fhttp_core::test_utils::root;

    use super::*;

    #[test]
    fn should_resolve_nested_dependencies() -> Result<()> {
        let root = root()
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("1.http");

        let init_request = Request::from_file(&init_path, false)?;

        let mut preprocessor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
            .parent().unwrap().join("resources/test/requests/duplicate_dependencies");
        let path1 = root.join("1.http");
        let path2 = root.join("2.http");
        let dep_path = root.join("dependency.http");

        let req1 = Request::from_file(&path1, false)?;
        let req2 = Request::from_file(&path2, false)?;

        let mut preprocessor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
        let req1 = Request::from_file(&path1, false).unwrap();

        Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
        let init_request = Request::from_file(&init_path, false).unwrap();

        let mut preprocessor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
            vec![init_request],
            Config::default()
        ).unwrap();

        preprocessor.next();
        preprocessor.next();
    }

    #[test]
    fn should_replace_dependencies_on_next_calls() -> Result<()> {
        let root = root()
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("4.http");
        let dep_path = root.join("5.http");

        let init_request = Request::from_file(&init_path, false)?;

        let mut preprocessor = Requestpreprocessor::new(
            Profile::empty(env::current_dir().unwrap()),
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
