use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::{Result, FhttpError, ErrorKind};
use std::env::{self, VarError};
use promptly::prompt;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profiles(HashMap<String, Profile>);

impl Profiles {
    pub fn parse(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let profiles = serde_json::from_str::<Profiles>(&content)?;

        Ok(profiles)
    }

    pub fn get(
        &self,
        key: &str
    ) -> Option<&Profile> {
        self.0.get(key)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Profile {
    variables: HashMap<String, String>,
}

impl Profile {
    pub fn new() -> Self {
        Profile {
            variables: HashMap::new(),
        }
    }

    pub fn get<K: Into<String>>(
        &self,
        key: K,
        prompt_for_missing: bool
    ) -> Result<String> {
        let key = key.into();

        if self.variables.contains_key(&key) {
            self.variables
                .get(&key)
                .map(|v| v.clone())
                .ok_or(FhttpError::new(ErrorKind::MissingEnvVar(key)))
        } else {
            match env::var(&key) {
                Ok(value) => Ok(value),
                Err(err) => match err {
                    VarError::NotPresent => match prompt_for_missing {
                        true => {
                            let value = prompt::<String, _>(&key);
                            env::set_var(&key, &value);
                            Ok(value)
                        },
                        false => Err(FhttpError::new(ErrorKind::MissingEnvVar(key)))
                    },
                    VarError::NotUnicode(_) => Err(FhttpError::new(ErrorKind::MissingEnvVar(key.into())))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;
    use maplit::hashmap;
    use std::env;

    #[test]
    fn should_load_profiles() -> Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/test/profiles/profile1.json");
        let profiles = Profiles::parse(&path)?;
        assert_eq!(
            profiles,
            Profiles(hashmap!{
                "development".into() => Profile {
                    variables: hashmap!{},
                },
                "testing".into() => Profile {
                    variables: hashmap!{
                        "var1".into() => "value1".into()
                    },
                }
            })
        );

        Ok(())
    }

    #[test]
    fn get_should_get_variables() -> Result<()> {
        let profile = Profile {
            variables: hashmap! {
                "a".into() => "b".into()
            },
        };

        assert_eq!(profile.get("a", false)?, String::from("b"));

        Ok(())
    }

    #[test]
    fn get_should_default_to_env_vars() -> Result<()> {
        env::set_var("a", "A");

        let profile = Profile {
            variables: HashMap::new(),
        };

        assert_eq!(profile.get("a", false)?, String::from("A"));

        Ok(())
    }
}
