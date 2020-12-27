extern crate assert_cmd;
extern crate futures;
extern crate wiremock;

use std::env;

use assert_cmd::Command;
use futures::executor::block_on;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use fhttp_core::test_utils::root;

#[test]
fn should_respect_print_paths_option() {
    block_on(async_test());
}

async fn async_test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Mock::given(method("GET"))
        .and(path("/1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/1.http")
        .arg("-P")
        .assert()
        .success()
        .stderr(format!("{}/resources/it/requests/1.http... 200 OK\n", root().to_str()));
}
