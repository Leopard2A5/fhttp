use std::collections::HashMap;
use std::env::{self, VarError};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use promptly::prompt;
use serde::{Deserialize, Serialize};

pub use profile_variable::ProfileVariable;

use crate::{Config, FhttpError, ResponseStore, Result};
use crate::path_utils::get_dependency_path;

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

    pub fn get<'a, K: Into<&'a str>>(
        &self,
        key: K,
        config: &Config,
        response_store: &ResponseStore,
        default: Option<&'a str>,
    ) -> Result<String> {
        let key = key.into();

        match self.variables.get(key) {
            Some(ProfileVariable::Request { request }) => Ok(response_store.get(&get_dependency_path(&self.source_path, request))),
            Some(var) => var.get(&config),
            None => get_from_environment(&key, config, default)
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn variables(&self) -> Vec<&ProfileVariable> {
        self.variables.values().collect()
    }

    pub fn override_with(
        &mut self,
        other: Profile
    ) {
        for (key, value) in other.variables {
            self.variables.insert(key, value);
        }
    }
}

fn get_from_environment(
    key: &str,
    config: &Config,
    default: Option<&str>,
) -> Result<String> {
    match env::var(&key) {
        Ok(value) => Ok(value),
        Err(VarError::NotUnicode(_)) => Err(FhttpError::new(format!("environment variable {} is not unicode!", key))),
        Err(VarError::NotPresent) => match default {
            Some(default) => Ok(default.to_owned()),
            None => match config.prompt_missing_env_vars() {
                true => {
                    let value = prompt::<String, _>(&key).unwrap();
                    env::set_var(&key, &value);
                    Ok(value)
                },
                false => Err(FhttpError::new(format!("missing environment variable {}", key)))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use maplit::hashmap;

    use crate::profiles::ProfileVariable;
    use crate::test_utils::root;

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

        assert_eq!(
            profile.get("a", &Config::default(), &ResponseStore::new(), None)?,
            String::from("b")
        );

        Ok(())
    }

    #[test]
    fn get_should_default_to_env_vars() -> Result<()> {
        env::set_var("a", "A");

        let profile = Profile {
            source_path: env::current_dir().unwrap(),
            variables: HashMap::new(),
        };

        assert_eq!(
            profile.get("a", &Config::default(), &ResponseStore::new(), None)?,
            String::from("A")
        );

        Ok(())
    }

    #[test]
    fn override_with_should_merge() -> Result<()> {
        let config = Config::default();
        let response_store = ResponseStore::new();

        let mut default = Profile::new(
            env::current_dir().unwrap(),
            hashmap! {
                String::from("a") => ProfileVariable::StringValue(String::from("A")),
                String::from("b") => ProfileVariable::StringValue(String::from("B"))
            }
        );
        let local = Profile::new(
            env::current_dir().unwrap(),
            hashmap! {
                String::from("b") => ProfileVariable::StringValue(String::from("BBB")),
                String::from("c") => ProfileVariable::StringValue(String::from("CCC")),
            }
        );

        default.override_with(local);
        assert_eq!(default.get("a", &config, &response_store, None)?, "A");
        assert_eq!(default.get("b", &config, &response_store, None)?, "BBB");
        assert_eq!(default.get("c", &config, &response_store, None)?, "CCC");

        Ok(())
    }
}
