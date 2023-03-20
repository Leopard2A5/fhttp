extern crate assert_cmd;
extern crate wiremock;
extern crate predicates;

use anyhow::Result;
use assert_cmd::Command;
use fhttp_test_utils::{write_test_file};
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[rstest]
async fn should_respect_print_paths_option() -> Result<()> {
    use predicates::prelude::*;

    let mock_server = MockServer::start().await;

    let workdir = TempDir::new()?;

    let request = write_test_file(
        &workdir,
        "request.txt",
        "GET ${env(URL)}/1\n"
    )?;

    Mock::given(method("GET"))
        .and(path("/1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .env("URL", mock_server.uri())
        .arg(request.to_str())
        .arg("-P")
        .assert()
        .success()
        .stderr(predicate::str::contains(format!("{}... 200 OK\n", request.to_str())));

    Ok(())
}
