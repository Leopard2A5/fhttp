use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use crate::{Result, FhttpError, ErrorKind};
use std::process::Command;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProfileVariable {
    StringValue(String),
    PassSecret {
        path: String,
        #[serde(skip)]
        cache: RefCell<Option<String>>
    },
    Request {
        request: String,
    },
}

impl ProfileVariable {

    pub fn get(
        &self
    ) -> Result<String> {
        match self {
            ProfileVariable::StringValue(ref value) => Ok(value.to_owned()),
            ProfileVariable::PassSecret { path, cache } => {
                if cache.borrow().is_none() {
                    let value = resolve_pass(&path)?.trim().to_owned();
                    cache.borrow_mut().replace(value);
                }

                Ok(cache.borrow().as_ref().unwrap().clone())
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
        Err(FhttpError::new(ErrorKind::ErrorInvokingProgram(
            format!("pass returned an error: '{}'", stderr).into()
        )))
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
                "path": "foo/bar"
            }
        "##);
        let result = serde_json::from_str::<ProfileVariable>(&input).unwrap();
        assert_eq!(result, ProfileVariable::PassSecret { path: "foo/bar".into(), cache: RefCell::new(None) });
    }

}
