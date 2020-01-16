use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProfileVariable {
    StringValue(String),
}

impl ProfileVariable {

    pub fn get(
        &self
    ) -> &str {
        match self {
            ProfileVariable::StringValue(ref value) => value
        }
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn serialize_string_value() {
        let foo = ProfileVariable::StringValue("foo".into());
        let result = serde_json::to_string(&foo).unwrap();
        assert_eq!(result, "\"foo\"");
    }

}
