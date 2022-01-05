extern crate assert_cmd;
extern crate async_std;
extern crate wiremock;

use std::env;
use std::time::Duration;

use assert_cmd::Command;
use async_std::task::block_on;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;

#[test]
fn should_apply_timeouts() {
    block_on(async_test());
}

async fn async_test() {
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("ok")
            .set_delay(Duration::from_millis(1_000)))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .arg("--timeout-ms").arg("500")
        .arg("../resources/it/requests/1.http")
        .assert()
        .failure()
        .stderr(format!(
            "GET {uri}/1... error sending request for url ({uri}/1): operation timed out\n",
            uri=mock_server.uri()
        ));
}
