use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use crate::{Result, FhttpError, Config};
use std::process::Command;

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
    ) -> Result<String> {
        match self {
            ProfileVariable::StringValue(ref value) => Ok(value.to_owned()),
            ProfileVariable::PassSecret { pass: path, cache } => {
                if config.curl() {
                    Ok(format!("$(pass {})", path))
                } else {
                    if cache.borrow().is_none() {
                        config.log(2, format!("resolving pass secret '{}'... ", &path));
                        let value = resolve_pass(&path)?.trim().to_owned();
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

fn resolve_pass(path: &str) -> Result<String> {
    let output = Command::new("pass")
        .args(&[path])
        .output()
        .unwrap();

    if output.status.success() {
        let output = output.stdout;
        Ok(String::from_utf8(output).unwrap())
    } else {
        let stderr = String::from_utf8(output.stderr).unwrap();
        Err(FhttpError::new(
            format!("pass returned an error: '{}'", stderr)
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

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

    static CONFIG: Config = Config::new(
        false,
        0,
        false,
        false,
        None,
        true,
    );

    #[test]
    fn string_value_should_return_normally() {
        let var = ProfileVariable::StringValue(String::from("value"));
        let result = var.get(&CONFIG);

        assert_eq!(result, Ok(String::from("value")));
    }

    #[test]
    fn pass_secret_should_return_pass_invocation() {
        let var = ProfileVariable::PassSecret { pass: "path/to/secret".to_string(), cache: RefCell::new(None) };
        let result = var.get(&CONFIG);

        assert_eq!(result, Ok(String::from("$(pass path/to/secret)")));
    }
}
