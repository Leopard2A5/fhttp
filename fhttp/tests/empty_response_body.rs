extern crate assert_cmd;
extern crate async_std;
extern crate wiremock;

use std::env;

use wiremock::{MockServer, ResponseTemplate, Mock};
use wiremock::matchers::method;
use assert_cmd::Command;
use async_std::task::block_on;

#[test]
fn test() {
    block_on(async_test());
}

async fn async_test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/empty_response_body.http")
        .assert()
        .failure()
        .stderr(format!(
            "POST {}/... Error: failed to parse response body as json\nBody was ''\n\nCaused by:\n    EOF while parsing a value at line 1 column 0\n",
            mock_server.uri()
        ));
}
