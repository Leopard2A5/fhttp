use std::fs;
use std::path::Path;

use fhttp_core::{Config, ResponseStore};
use fhttp_core::{Request};
use fhttp_core::VariableSupport;
use fhttp_core::Result;
use fhttp_core::execution_order::plan_request_order;
use fhttp_core::{Profile};

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
}

impl Iterator for Requestpreprocessor {
    type Item = Result<Request>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.requests.len() > 0 {
            let mut req = self.requests.remove(0);

            let req = req.replace_variables(&self.profile, &self.config, &self.response_data)
                .map(|_| req);
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

    use fhttp_core::Request;
    use fhttp_core::test_utils::root;

    use super::*;

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
