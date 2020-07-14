extern crate assert_cmd;
extern crate wiremock;
extern crate futures;

use std::env;

use assert_cmd::Command;
use wiremock::{MockServer, Mock, ResponseTemplate};
use futures::executor::block_on;
use wiremock::matchers::{method, path, body_string_contains};

#[test]
fn test() {
    block_on(async_test());
}

async fn async_test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Content-Disposition: form-data; name=\"file\"; filename=\"multipart_content_1.txt\""))
        .and(body_string_contains("Content-Type: text/plain"))
        .and(body_string_contains("this is a file"))
        .and(body_string_contains("Content-Disposition: form-data; name=\"data\"; filename=\"multipart_content_2.txt\""))
        .and(body_string_contains("this is another file"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/multipart.http")
        .assert()
        .success();
}
