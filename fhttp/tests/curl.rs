extern crate assert_cmd;
extern crate mockito;
extern crate reqwest;

use std::env;

use assert_cmd::Command;
use futures::executor::block_on;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, body_string_contains};

use fhttp_core::curl::Curl;
use fhttp_core::request::Request;
use fhttp_core::request_def::body::Body;

#[test]
fn curl_invocation_with_dependency() {
    block_on(test())
}

async fn test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());
    env::set_var("USERNAME", "username_secret");
    env::set_var("PASSWORD", "password_secret");

    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("username_secret"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(r#"{ "token": "jwt" }"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_static("Bearer jwt")
    );
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json")
    );
    let req = Request {
        method: Method::POST,
        url: format!("{}/resources", mock_server.uri()),
        headers,
        body: Body::Plain("{\n  \"name\": \"resource\"\n}".to_string()) ,
        response_handler: None
    };
    let req = req.curl();

    Command::cargo_bin("fhttp").unwrap()
        .arg("--curl")
        .arg("../resources/it/curl/create.http")
        .assert()
        .success()
        .stderr(format!(
            "POST {uri}/token... 200 OK\nPOST {uri}/resources... ",
            uri=mock_server.uri(),
        ))
        .stdout(format!(
            "{curl}\n",
            curl=req,
        ));
}
