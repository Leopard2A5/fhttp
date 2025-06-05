use std::fmt::Debug;
use anyhow::{bail, format_err, Context, Result};
use rhai::{Engine, Scope};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ResponseHandler {
    Json { json_path: String },
    Deno { program: String },
    Rhai { program: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResponseHandlerInput {
    pub status_code: u16,
    pub body: String,
}

impl ResponseHandler {
    pub fn process_body(&self, input: ResponseHandlerInput) -> Result<String> {
        match self {
            ResponseHandler::Json { json_path } => process_body_json(json_path, &input.body),
            ResponseHandler::Deno { .. } => {
                bail!("deno response handlers are no longer supported.")
            }
            ResponseHandler::Rhai { program } => process_response_rhai(program, input),
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

fn process_response_rhai(program: &String, response: ResponseHandlerInput) -> Result<String> {
    let ResponseHandlerInput { status_code, body } = response;
    let engine = Engine::new();
    let mut scope = Scope::new();

    scope.push("status", status_code as i64); // use i64 for seamless comparisons in-script
    scope.push("body", body);

    engine.eval_with_scope::<String>(&mut scope, program)
        .map_err(|e| format_err!("{}", e))
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
        ).to_string();
        let handler = ResponseHandler::Json {
            json_path: "$.a.b.c".into(),
        };
        let result = handler.process_body(ResponseHandlerInput{ body, status_code: 200 });

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
        ).to_string();
        let handler = ResponseHandler::Json {
            json_path: "$.a.b.c".into(),
        };
        let result = handler.process_body(ResponseHandlerInput { body, status_code: 200 });

        assert_ok!(result, String::from("3.141"));
    }
}

#[cfg(test)]
mod deno_tests {
    use super::*;

    #[test]
    fn should_not_support_deno_anymore() {
        let body = "this is the response body".to_string();
        let handler = ResponseHandler::Deno { program: "".into() };
        let result = handler.process_body(ResponseHandlerInput { body, status_code: 200 });

        assert_err!(result, "deno response handlers are no longer supported.");
    }
}

#[cfg(test)]
mod rhai_tests {
    use indoc::indoc;
    use insta::{assert_debug_snapshot};
    use super::*;

    #[test]
    fn should_pass_in_the_status_code() {
        let input = ResponseHandlerInput {
            body: "".to_string(),
            status_code: 200,
        };
        let handler = ResponseHandler::Rhai {
            program: indoc! ("
                if status == 200 {
                    return \"true\";
                } else {
                    return \"false\"
                }
            ").to_string(),
        };
        let result = handler.process_body(input).expect("failed to invoke handler");
        assert_debug_snapshot!(result, @r#""true""#);
    }
    
    #[test]
    fn should_pass_in_the_body() {
        let input = ResponseHandlerInput {
            body: "hello".to_string(),
            status_code: 200,
        };
        let handler = ResponseHandler::Rhai {
            program: indoc! ("
                body + \", world!\"
            ").to_string(),
        };
        let result = handler.process_body(input).expect("failed to invoke handler");
        assert_debug_snapshot!(result, @r#""hello, world!""#);
    }
}
