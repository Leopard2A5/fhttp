extern crate assert_cmd;
extern crate futures;
extern crate wiremock;

use std::env;

use futures::executor::block_on;
use wiremock::{MockServer, ResponseTemplate, Mock};
use wiremock::matchers::method;
use assert_cmd::Command;

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
            "POST {}/... Error parsing response body as json: EOF while parsing a value at line 1 column 0. Body was ''\n",
            mock_server.uri()
        ));
}
