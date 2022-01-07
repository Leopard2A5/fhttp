extern crate assert_cmd;
extern crate mockito;
extern crate reqwest;
extern crate async_std;

use std::env;

use assert_cmd::Command;
use async_std::task::block_on;
use wiremock::MockServer;

use fhttp_core::test_utils::root;

#[test]
fn should_show_error_when_asked_to_output_dependencies() {
    block_on(test())
}

async fn test() {
    let root = root();
    let mock_server = MockServer::start().await;
    env::set_var("URL", mock_server.uri());

    Command::cargo_bin("fhttp").unwrap()
        .arg("--curl")
        .arg("../resources/it/curl/token.http")
        .arg("../resources/it/curl/create.http")
        .assert()
        .failure()
        .stderr(format!(
            "Error: {}\nis a dependency of\n{}.\nIf you want me to print the curl snippet for both requests you'll need to do them separately.\n",
            root.join("resources/it/curl/token.http").to_str(),
            root.join("resources/it/curl/create.http").to_str(),
        ));
}
