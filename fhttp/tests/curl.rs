extern crate assert_cmd;
extern crate mockito;
extern crate reqwest;

use anyhow::Result;
use assert_cmd::Command;
use async_std::task::block_on;
use fhttp_core::execution::curl::Curl;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_core::request::body::Body;
use fhttp_core::request::Request;
use fhttp_test_utils::write_test_file;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use temp_dir::TempDir;
use std::env;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use indoc::{indoc, formatdoc};

#[test]
fn curl_invocation_with_dependency() -> Result<()> {
    block_on(test())
}

async fn test() -> Result<()> {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());
    env::set_var("USERNAME", "username_secret");
    env::set_var("PASSWORD", "password_secret");

    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("username_secret"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{ "token": "jwt" }"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_static("Bearer jwt"),
    );
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );
    let body = "{\n  \"name\": \"resource\"\n}".to_string();
    let expectation = Request {
        method: Method::POST,
        url: format!("{}/resources", mock_server.uri()),
        headers,
        body: Body::Plain(body.clone()),
        response_handler: None,
    }.curl();

    let workdir = TempDir::new()?;

    write_test_file(
        &workdir,
        "token.http",
        indoc!("
            POST ${env(URL)}/token
            Content-Type: application/json
            
            {
                \"username\": \"${env(USERNAME)}\",
                \"password\": \"${env(PASSWORD)}\"
            }
            
            > {%
                json $.token
            %}
        ")
    )?;

    let req = write_test_file(
        &workdir,
        "req.http",
        &formatdoc!("
            POST ${{env(URL)}}/resources
            Authorization: Bearer ${{request(\"token.http\")}}
            Content-Type: application/json

            {body}

            > {{%
                json $.id
            %}}
        ", body = body)
    )?;

    let workdir = CanonicalizedPathBuf::new(workdir.path());
    let req = workdir.join(req);

    Command::cargo_bin("fhttp")
        .unwrap()
        .arg("--curl")
        .arg(req.to_str())
        .assert()
        .success()
        .stderr(format!(
            "POST {uri}/token... 200 OK\nPOST {uri}/resources... ",
            uri = mock_server.uri(),
        ))
        .stdout(format!("\n{curl}\n", curl = expectation));

    Ok(())
}
