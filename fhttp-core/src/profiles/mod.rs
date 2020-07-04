use std::collections::HashMap;
use std::env::{self, VarError};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use promptly::prompt;
use serde::{Deserialize, Serialize};

use crate::{FhttpError, Result, Config};
pub use profile_variable::ProfileVariable;

mod profile_variable;

pub struct Profiles;

impl Profiles {
    pub fn parse(path: &Path) -> Result<HashMap<String, Profile>> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| FhttpError::new(format!("Error opening file {}", path.to_str().unwrap())))?;
        let profiles = serde_json::from_str::<HashMap<String, _Profile>>(&content)
            .map_err(|_| FhttpError::new(format!("error reading profile from {}", path.to_str().unwrap())))?;
        let ret = profiles.into_iter()
            .map(|(key, value)| {
                let profile = Profile::new(path, value.variables);
                (key, profile)
            })
            .collect::<HashMap<String, Profile>>();

        Ok(ret)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
struct _Profile {
    pub variables: HashMap<String, ProfileVariable>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Profile {
    source_path: PathBuf,
    variables: HashMap<String, ProfileVariable>,
}

impl Profile {
    pub fn empty<T: Into<PathBuf>>(source_path: T) -> Self {
        Profile {
            source_path: source_path.into(),
            variables: HashMap::new(),
        }
    }

    pub fn new<T: Into<PathBuf>>(
        source_path: T,
        variables: HashMap<String, ProfileVariable>,
    ) -> Self {
        Profile {
            source_path: source_path.into(),
            variables,
        }
    }

    pub fn defined_through_request<K: Into<String>>(
        &self,
        key: K
    ) -> Option<PathBuf> {
        let key = key.into();

        match self.variables.contains_key(&key) {
            true => match self.variables.get(&key) {
                Some(ProfileVariable::Request { request }) => Some(PathBuf::from_str(request).unwrap()),
                _ => None
            },
            false => None
        }
    }

    pub fn get<K: Into<String>>(
        &self,
        key: K,
        config: &Config,
    ) -> Result<Resolve> {
        let key = key.into();

        if self.variables.contains_key(&key) {
            match self.variables.get(&key) {
                Some(variable) => match variable {
                    ProfileVariable::StringValue(_) => Ok(Resolve::StringValue(variable.get()?)),
                    ProfileVariable::PassSecret { cache: _, path: _ } => Ok(Resolve::StringValue(variable.get()?)),
                    ProfileVariable::Request { request } => Ok(Resolve::RequestLookup(PathBuf::from_str(request).unwrap())),
                },
                None => Err(FhttpError::new(format!("missing environment variable {}", key)))
            }
        } else {
            match env::var(&key) {
                Ok(value) => Ok(Resolve::StringValue(value)),
                Err(err) => match err {
                    VarError::NotPresent => match config.prompt_missing_env_vars {
                        true => {
                            let value = prompt::<String, _>(&key).unwrap();
                            env::set_var(&key, &value);
                            Ok(Resolve::StringValue(value))
                        },
                        false => Err(FhttpError::new(format!("missing environment variable {}", key)))
                    },
                    VarError::NotUnicode(_) => Err(FhttpError::new(format!("missing environment variable {}", key)))
                }
            }
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn variables(&self) -> Vec<&ProfileVariable> {
        self.variables.values().collect()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Resolve {
    StringValue(String),
    RequestLookup(PathBuf),
}

#[cfg(test)]
mod test {
    use std::env;

    use maplit::hashmap;

    use crate::test_utils::root;

    use crate::profiles::ProfileVariable;

    use super::*;

    #[test]
    fn should_load_profiles() -> Result<()> {
        let path = root()
            .join("resources/test/profiles/profile1.json");
        let profiles = Profiles::parse(&path)?;
        assert_eq!(
            profiles,
            hashmap!{
                "development".into() => Profile {
                    source_path: root().join("resources/test/profiles/profile1.json"),
                    variables: hashmap!{},
                },
                "testing".into() => Profile {
                    source_path: root().join("resources/test/profiles/profile1.json"),
                    variables: hashmap!{
                        "var1".into() => ProfileVariable::StringValue("value1".into())
                    },
                }
            }
        );

        Ok(())
    }

    #[test]
    fn get_should_get_variables() -> Result<()> {
        let profile = Profile {
            source_path: env::current_dir().unwrap(),
            variables: hashmap! {
                "a".into() => ProfileVariable::StringValue("b".into())
            },
        };

        assert_eq!(profile.get("a", &Config::default())?, Resolve::StringValue(String::from("b")));

        Ok(())
    }

    #[test]
    fn get_should_default_to_env_vars() -> Result<()> {
        env::set_var("a", "A");

        let profile = Profile {
            source_path: env::current_dir().unwrap(),
            variables: HashMap::new(),
        };

        assert_eq!(profile.get("a", &Config::default())?, Resolve::StringValue(String::from("A")));

        Ok(())
    }
}
