use std::fmt::Debug;

#[derive(Debug)]
pub enum ResponseHandler {
    Json { json_path: String },
}

impl ResponseHandler {
    pub fn process_body(
        &self,
        body: &str
    ) -> String {
        match self {
            ResponseHandler::Json { json_path } => process_body_json(json_path, body)
        }
    }
}

fn process_body_json(
    json_path: &str,
    body: &str
) -> String {
    use jsonpath::Selector;
    use serde_json::Value;

    let value: Value = serde_json::from_str(body).unwrap();

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
        Value::String(string) => string,
        _ => serde_json::to_string(&result).unwrap()
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use indoc::indoc;

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
        let result = handler.process_body(body);

        assert_eq!(result, "success");
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
        let result = handler.process_body(body);

        assert_eq!(result, "3.141");
    }
}