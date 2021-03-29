use crate::parsers::Request;
use crate::request_def::body::Body;
use serde_json::Value;

pub trait Curl {
    fn curl(&self) -> String;
}

impl Curl for Request {
    fn curl(&self) -> String {
        let mut parts = vec![
            format!("curl -X {}", self.method.as_str()),
        ];

        for (name, value) in self.headers.iter() {
            parts.push(format!(
                "-H \"{}: {}\"",
                name.as_str().replace(r#"""#, r#"\""#),
                value.to_str().unwrap().replace(r#"""#, r#"\""#)
            ))
        }

        match &self.body {
            Body::Plain(body) => if !body.is_empty() {
                parts.push(format!(
                    "-d {}",
                    escape_json(body)
                ));
            },
            Body::Files(files) => for file in files {
                parts.push(format!(
                    "-F \"{}=@{}\"",
                    file.name.replace(r#"""#, r#"\""#),
                    file.path.to_str().replace(r#"""#, r#"\""#)
                ));
            },
        }

        parts.push(format!("--url \"{}\"", &self.url.replace(r#"""#, r#"\""#)));

        parts.join(" \\\n")
    }
}

fn escape_json<S: Into<String>>(input: S) -> String {
    serde_json::to_string(
        &Value::String(input.into())
    ).unwrap()
}

#[cfg(test)]
mod test {
    use indoc::{indoc, formatdoc};

    use crate::test_utils::root;

    use serde_json::json;

    use super::*;

    #[test]
    fn should_print_command_for_simple_request() {
        let result = Request::basic("GET", "http://localhost/123")
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                --url "http://localhost/123""#
            )
        );
    }

    #[test]
    fn should_print_command_with_headers() {
        let result = Request::basic("GET", "http://localhost/123")
            .add_header("accept", "application/json")
            .add_header("content-type", "application/json")
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "accept: application/json" \
                -H "content-type: application/json" \
                --url "http://localhost/123""#
            )
        );
    }

    #[test]
    fn should_print_command_with_headers_and_body() {
        let body = "{\n    \"foo\": \"bar\",\n    \"bar\": \"escape'me\"}";

        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .body(&body)
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -d "{\n    \"foo\": \"bar\",\n    \"bar\": \"escape'me\"}" \
                --url "http://localhost/555""#
            )
        );
    }

    #[test]
    fn should_print_command_with_plain_text_body() {
        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .body("this is a so-called \"test\"")
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -d "this is a so-called \"test\"" \
                --url "http://localhost/555""#
            )
        );
    }

    #[test]
    fn should_print_command_with_gql_body() {
        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .gql_body(
                json!({
                    "query": "query { search(filter: \"foobar\") { id } }",
                })
            )
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -d "{\"query\":\"query { search(filter: \\\"foobar\\\") { id } }\"}" \
                --url "http://localhost/555""#
            )
        );
    }

    #[test]
    fn should_print_command_with_headers_and_files() {
        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .file_body(&[
                ("file1", "resources/it/profiles.json"),
                ("file2", "resources/it/profiles2.json"),
            ])
            .curl();

        assert_eq!(
            result,
            formatdoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -F "file1=@{base}/resources/it/profiles.json" \
                -F "file2=@{base}/resources/it/profiles2.json" \
                --url "http://localhost/555""#,
                base = root().to_str().to_string(),
            )
        );
    }
}
