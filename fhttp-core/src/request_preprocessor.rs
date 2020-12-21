use std::fs;
use std::path::Path;

use crate::Config;
use crate::ResponseStore;
use crate::Request;
use crate::VariableSupport;
use crate::Result;
use crate::execution_order::plan_request_order;
use crate::Profile;

// #[derive(Debug)]
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
mod dependencies {
    use std::env;

    use crate::Request;
    use crate::test_utils::root;

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
