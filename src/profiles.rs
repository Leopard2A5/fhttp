use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::Result;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profiles(HashMap<String, Profile>);

impl Profiles {
    pub fn parse(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let profiles = serde_json::from_str::<Profiles>(&content)?;

        Ok(profiles)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    variables: HashMap<String, String>,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;
    use maplit::hashmap;

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

}
