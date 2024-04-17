use std::fmt::Debug;

use anyhow::{bail, Context, Result};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ResponseHandler {
    Json { json_path: String },
    Deno { program: String },
}

impl ResponseHandler {
    pub fn process_body(&self, body: &str) -> Result<String> {
        match self {
            ResponseHandler::Json { json_path } => process_body_json(json_path, body),
            ResponseHandler::Deno { .. } => {
                bail!("deno response handlers are no longer supported.")
            }
        }
    }
}

fn process_body_json(json_path: &str, body: &str) -> Result<String> {
    use jsonpath_lib::Selector;
    use serde_json::Value;

    let value: Value = serde_json::from_str(body)
        .with_context(|| format!("failed to parse response body as json\nBody was '{}'", body))?;

    let mut selector = Selector::new();
    let json_path_results = selector
        .str_path(json_path)
        .unwrap()
        .value(&value)
        .select()
        .unwrap();
    let result = match json_path_results.len() {
        0 => Value::String("".into()),
        _ => json_path_results[0].clone(),
    };

    match result {
        Value::String(string) => Ok(string),
        _ => Ok(serde_json::to_string(&result).unwrap()),
    }
}

#[cfg(test)]
mod json_tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn should_apply_the_jsonpath_expression() {
        let body = indoc!(
            "
            {
                \"a\": {
                    \"b\": {
                        \"c\": \"success\"
                    },
                    \"c\": \"failure\"
                }
            }
        "
        );
        let handler = ResponseHandler::Json {
            json_path: "$.a.b.c".into(),
        };
        let result = handler.process_body(body);

        assert_ok!(result, String::from("success"));
    }

    #[test]
    fn should_convert_numbers_to_string() {
        let body = indoc!(
            "
            {
                \"a\": {
                    \"b\": {
                        \"c\": 3.141
                    }
                }
            }
        "
        );
        let handler = ResponseHandler::Json {
            json_path: "$.a.b.c".into(),
        };
        let result = handler.process_body(body);

        assert_ok!(result, String::from("3.141"));
    }
}

#[cfg(test)]
mod deno_tests {
    use super::*;

    #[test]
    fn should_not_support_deno_anymore() {
        let body = "this is the response body";
        let handler = ResponseHandler::Deno { program: "".into() };
        let result = handler.process_body(body);

        assert_err!(result, "deno response handlers are no longer supported.");
    }
}
