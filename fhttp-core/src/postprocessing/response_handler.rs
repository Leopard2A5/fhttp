use std::collections::HashMap;
use std::fmt::Debug;

use crate::errors::{FhttpError, Result};
use deno_core::{RuntimeOptions, Snapshot};

static FHTTP_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/FHTTP_SNAPSHOT.bin"));

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ResponseHandler {
    Json { json_path: String },
    Deno { program: String },
}

impl ResponseHandler {
    pub fn process_body(
        &self,
        status: u16,
        headers: &HashMap<&str, &str>,
        body: &str,
    ) -> Result<String> {
        match self {
            ResponseHandler::Json { json_path } => process_body_json(json_path, body),
            ResponseHandler::Deno { program } => process_body_deno(program, status, headers, body),
        }
    }
}

fn process_body_json(
    json_path: &str,
    body: &str
) -> Result<String> {
    use jsonpath::Selector;
    use serde_json::Value;

    let value: Value = serde_json::from_str(body)
        .map_err(|e| FhttpError::new(format!(
            "Error parsing response body as json: {}. Body was '{}'",
            e.to_string(),
            body
        )))?;

    let mut selector = Selector::new();
    let json_path_results = selector
        .str_path(json_path).unwrap()
        .value(&value)
        .select()
        .unwrap();
    let result = match json_path_results.len() {
        0 => Value::String("".into()),
        _ => json_path_results[0].clone(),
    };

    match result {
        Value::String(string) => Ok(string),
        _ => Ok(serde_json::to_string(&result).unwrap())
    }
}

fn process_body_deno(
    program: &str,
    status: u16,
    headers: &HashMap<&str, &str>,
    body: &str,
) -> Result<String> {
    use deno_core::op_sync;
    use deno_core::JsRuntime;
    use std::cell::Cell;
    use std::rc::Rc;

    let result: Rc<Cell<String>> = Rc::new(Cell::new(body.to_string()));
    let runtime_options = RuntimeOptions {
        startup_snapshot: Some(Snapshot::Static(FHTTP_SNAPSHOT)),
        ..RuntimeOptions::default()
    };
    let mut runtime = JsRuntime::new(runtime_options);

    let result_ref = result.clone();
    runtime.register_op(
        "op_set_result",
        op_sync(move |_state, result: String, _: ()| {
            result_ref.set(result);
            Ok(())
        }),
    );
    runtime.sync_ops_cache();

    let program = prepare_deno_code(program, status, headers, body);

    runtime
        .execute_script("", &program)
        .unwrap();

    Ok(result.take())
}

fn prepare_deno_code(
    program: &str,
    status: u16,
    headers: &HashMap<&str, &str>,
    body: &str,
) -> String {
    let header_lines = headers.iter()
        .map(|(name, value)|
            format!(
                "    '{}': '{}'",
                name.replace("'", "\\'"),
                value.replace("'", "\\'")
            )
        )
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        r#"
            const status = {status};
            const headers = {{
                {headers}
            }};
            const body = '{body}';

            {program}
        "#,
        status = status,
        body = &body.replace("'", "\\'"),
        headers = &header_lines,
        program = &program,
    )
}

#[cfg(test)]
mod json_tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn should_apply_the_jsonpath_expression() {
        let body = indoc!("
            {
                \"a\": {
                    \"b\": {
                        \"c\": \"success\"
                    },
                    \"c\": \"failure\"
                }
            }
        ");
        let handler = ResponseHandler::Json { json_path: "$.a.b.c".into() };
        let result = handler.process_body(200, &HashMap::new(), body);

        assert_eq!(result, Ok(String::from("success")));
    }

    #[test]
    fn should_convert_numbers_to_string() {
        let body = indoc!("
            {
                \"a\": {
                    \"b\": {
                        \"c\": 3.141
                    }
                }
            }
        ");
        let handler = ResponseHandler::Json { json_path: "$.a.b.c".into() };
        let result = handler.process_body(200, &HashMap::new(), body);

        assert_eq!(result, Ok(String::from("3.141")));
    }
}

#[cfg(test)]
mod deno_tests {
    use maplit::hashmap;

    use super::*;

    #[test]
    fn should_default_to_response_body_as_result() {
        let body = "this is the response body";
        let handler = ResponseHandler::Deno { program: String::new() };
        let result = handler.process_body(200, &HashMap::new(), body);

        assert_eq!(result, Ok(String::from("this is the response body")));
    }

    #[test]
    fn should_allow_setting_result() {
        let body = "this is the response body";
        let handler = ResponseHandler::Deno {
            program: r#"
                setResult(body.toUpperCase());
            "#.into()
        };
        let result = handler.process_body(200, &HashMap::new(), body);

        assert_eq!(result, Ok(String::from("THIS IS THE RESPONSE BODY")));
    }

    #[test]
    fn should_have_access_to_result_headers() {
        let body = "this is the response body";
        let headers = hashmap!{
            "content-type" => "application/json",
            "accept" => "application/json,application/xml"
        };
        let handler = ResponseHandler::Deno {
            program: r#"
                setResult(headers['accept']);
            "#.into()
        };
        let result = handler.process_body(200, &headers, body);

        assert_eq!(result, Ok(String::from("application/json,application/xml")));
    }

    #[test]
    fn should_escape_headers() {
        let body = "this is the response body";
        let headers = hashmap!{
            "content'type" => "appli'cation",
        };
        let handler = ResponseHandler::Deno {
            program: r#"
                setResult(headers['content\'type']);
            "#.into()
        };
        let result = handler.process_body(200, &headers, body);

        assert_eq!(result, Ok(String::from("appli'cation")));
    }

    #[test]
    fn should_escape_body() {
        let body = "this is the 'response' body";
        let headers = hashmap!{};
        let handler = ResponseHandler::Deno {
            program: r#"
                setResult(body);
            "#.into()
        };
        let result = handler.process_body(200, &headers, body);

        assert_eq!(result, Ok(String::from("this is the 'response' body")));
    }

    #[test]
    fn should_have_access_to_status_code() {
        let body = "this is the response body";
        let headers = hashmap!{};
        let handler = ResponseHandler::Deno {
            program: r#"
                if (status === 404)
                    setResult('not found');
                else if (status === 200)
                    setResult('ok');
                else
                    setResult('who knows?');
            "#.into()
        };
        let result = handler.process_body(200, &headers, body);

        assert_eq!(result, Ok(String::from("ok")));
    }

}
