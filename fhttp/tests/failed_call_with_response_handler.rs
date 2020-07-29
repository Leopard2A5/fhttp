extern crate assert_cmd;
extern crate futures;
extern crate wiremock;

use std::env;

use futures::executor::block_on;
use wiremock::{MockServer, ResponseTemplate, Mock};
use wiremock::matchers::method;
use assert_cmd::Command;

#[test]
fn should_not_execute_response_handler_on_unsuccessful_requests() {
    block_on(async_test());
}

async fn async_test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/empty_response_body.http")
        .assert()
        .failure()
        .stderr(format!(
            "POST {}/... 500 Internal Server Error\nno response body\n",
            mock_server.uri()
        ));
}
