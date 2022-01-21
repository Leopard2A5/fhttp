use crate::parsers::Request;
use crate::request::body::{Body, MultipartPart};

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
                    "-d \"{}\"",
                    escape_body(body)
                ));
            },
            Body::Multipart(multiparts) => for prt in multiparts {
                parts.push(match prt {
                    MultipartPart::File { name, file_path, mime_str } => {
                        let type_and_end = match mime_str {
                            None => "\"".to_string(),
                            Some(mime) => format!("; type={}\"", mime),
                        };
                        format!(
                            "-F {name}=\"@{filepath}{type_and_end}",
                            name = name,
                            filepath = file_path.to_str(),
                            type_and_end = type_and_end,
                        )
                    },
                    MultipartPart::Text { name, text, mime_str } => {
                        let type_and_end = match mime_str {
                            None => "\"".to_string(),
                            Some(mime) => format!("; type={}\"", mime),
                        };
                        format!(
                            "-F {name}=\"{text}{type_and_end}",
                            name = name,
                            text = text.replace('"', "\\\""),
                            type_and_end = type_and_end,
                        )
                    },
                });
            },
        }

        parts.push(format!("--url \"{}\"", &self.url.replace(r#"""#, r#"\""#)));

        parts.join(" \\\n")
    }
}

fn escape_body<S: Into<String>>(input: S) -> String {
    input.into()
        .replace("\n", "\\\n")
        .replace("\"", "\\\"")
}

#[cfg(test)]
mod test {
    use indoc::{formatdoc, indoc};
    use serde_json::json;

    use crate::request::body::MultipartPart;
    use crate::test_utils::root;

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
        let body = "{\n    \"foo\": \"bar\",\n    \"bar\": \"escape'me\"\n}";

        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .body(&body)
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -d "{\
                    \"foo\": \"bar\",\
                    \"bar\": \"escape'me\"\
                }" \
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
    fn should_print_command_with_body_with_newlines() {
        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .body("one\ntwo\nthree")
            .curl();

        assert_eq!(
            result,
            indoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -d "one\
                two\
                three" \
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
                -d "{\"query\":\"query { search(filter: \\"foobar\\") { id } }\"}" \
                --url "http://localhost/555""#
            )
        );
    }

    #[test]
    fn should_print_command_with_headers_and_files() {
        let result = Request::basic("GET", "http://localhost/555")
            .add_header("content-type", "application/json")
            .multipart(&[
                MultipartPart::File {
                    name: "file1".to_string(),
                    file_path: root().join("resources/it/profiles.json"),
                    mime_str: None,
                },
                MultipartPart::File {
                    name: "file2".to_string(),
                    file_path: root().join("resources/it/profiles2.json"),
                    mime_str: None,
                },
            ])
            .curl();

        assert_eq!(
            result,
            formatdoc!(r#"
                curl -X GET \
                -H "content-type: application/json" \
                -F file1="@{base}/resources/it/profiles.json" \
                -F file2="@{base}/resources/it/profiles2.json" \
                --url "http://localhost/555""#,
                base = root().to_str().to_string(),
            )
        );
    }

    #[test]
    fn should_print_command_with_multiparts() {
        let filepath = root().join("resources/image.jpg");
        let result = Request::basic("GET", "http://localhost/555")
            .multipart(&[
                MultipartPart::Text {
                    name: "textpart1".to_string(),
                    text: "this is a text".to_string(),
                    mime_str: Some("plain/text".to_string()),
                },
                MultipartPart::Text {
                    name: "textpart2".to_string(),
                    text: "{\"a\": 5}".to_string(),
                    mime_str: Some("application/json".to_string()),
                },
                MultipartPart::Text {
                    name: "textpart3".to_string(),
                    text: "this is a text".to_string(),
                    mime_str: None,
                },
                MultipartPart::File {
                    name: "filepart".to_string(),
                    file_path: filepath.clone(),
                    mime_str: None
                },
            ])
            .curl();

        assert_eq!(
            result,
            formatdoc!(r#"
                curl -X GET \
                -F textpart1="this is a text; type=plain/text" \
                -F textpart2="{{\"a\": 5}}; type=application/json" \
                -F textpart3="this is a text" \
                -F filepart="@{filepath}" \
                --url "http://localhost/555""#,
                filepath = filepath.to_str(),
            )
        );
    }
}
