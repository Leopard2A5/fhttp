use std::cell::RefCell;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::Config;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProfileVariable {
    StringValue(String),
    PassSecret {
        pass: String,
        #[serde(skip)]
        cache: RefCell<Option<String>>
    },
    Request {
        request: String,
    },
}

impl ProfileVariable {

    pub fn get(
        &self,
        config: &Config,
        for_dependency: bool,
    ) -> Result<String> {
        match self {
            ProfileVariable::StringValue(ref value) => Ok(value.to_owned()),
            ProfileVariable::PassSecret { pass: path, cache } => {
                if config.curl() && !for_dependency {
                    Ok(format!("$(pass {})", path))
                } else {
                    if cache.borrow().is_none() {
                        config.log(2, format!("resolving pass secret '{}'... ", &path));
                        let value = resolve_pass(path)?.trim().to_owned();
                        config.logln(2, "done");
                        cache.borrow_mut().replace(value);
                    }

                    Ok(cache.borrow().as_ref().unwrap().clone())
                }
            }
            ProfileVariable::Request { request: _ } => panic!("ProfileVariable::Request cannot resolve by itself"),
        }
    }

}

#[cfg(test)]
thread_local!(
    static PASS_INVOCATIONS: RefCell<Vec<String>> = RefCell::new(Vec::new())
);

#[cfg(test)]
fn resolve_pass(path: &str) -> Result<String> {
    PASS_INVOCATIONS.with(|it| it.borrow_mut().push(path.to_string()));
    Ok("pass_secret".to_string())
}

#[cfg(not(test))]
fn resolve_pass(path: &str) -> Result<String> {
    use std::process::Command;
    use anyhow::anyhow;

    let output = Command::new("pass")
        .args(&[path])
        .output()
        .unwrap();

    if output.status.success() {
        let output = output.stdout;
        Ok(String::from_utf8(output).unwrap())
    } else {
        let stderr = String::from_utf8(output.stderr).unwrap();
        Err(anyhow!("pass returned an error: '{}'", stderr))
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::*;

    #[test]
    fn deserialize_string_value() {
        let input = "\"foo\"";
        let result = serde_json::from_str::<ProfileVariable>(&input).unwrap();
        assert_eq!(result, ProfileVariable::StringValue("foo".into()));
    }

    #[test]
    fn deserialize_pass_secret() {
        let input = indoc!(r##"
            {
                "pass": "foo/bar"
            }
        "##);
        let result = serde_json::from_str::<ProfileVariable>(&input).unwrap();
        assert_eq!(result, ProfileVariable::PassSecret { pass: "foo/bar".into(), cache: RefCell::new(None) });
    }

}

#[cfg(test)]
mod curl {
    use super::*;
    use rstest::{rstest, fixture};

    #[fixture]
    fn program() -> Config {
        Config::new(
            false,
            0,
            false,
            false,
            None,
            true,
        )
    }

    #[rstest]
    fn string_value_should_return_normally(program: Config) {
        let var = ProfileVariable::StringValue(String::from("value"));
        let result = var.get(&program, false);

        assert_ok!(result, String::from("value"));
    }

    #[rstest]
    fn pass_should_return_pass_invocation_string_for_non_dependencies(program: Config) {
        PASS_INVOCATIONS.with(|it| it.borrow_mut().clear());

        let var = ProfileVariable::PassSecret { pass: "path/to/secret".to_string(), cache: RefCell::new(None) };
        let result = var.get(&program, false);

        assert_ok!(result, String::from("$(pass path/to/secret)"));

        PASS_INVOCATIONS.with(|it| assert_eq!(it.borrow().len(), 0));
    }

    #[rstest]
    fn pass_should_invoke_pass_for_dependencies(program: Config) {
        PASS_INVOCATIONS.with(|it| it.borrow_mut().clear());

        let var = ProfileVariable::PassSecret { pass: "path/to/secret".to_string(), cache: RefCell::new(None) };
        let result = var.get(&program, true);

        assert_ok!(result, String::from("pass_secret"));

        PASS_INVOCATIONS.with(|it| {
            let invocations = it.borrow().iter()
                .map(String::clone)
                .collect::<Vec<_>>();
            assert_eq!(&invocations, &["path/to/secret".to_string()]);
        });
    }
}
