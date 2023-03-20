extern crate assert_cmd;
extern crate wiremock;

use std::time::Duration;

use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use indoc::formatdoc;
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;

#[rstest]
async fn should_apply_timeouts() {
    let workdir = TempDir::new().unwrap();
    let mock_server = MockServer::start().await;
    let url = mock_server.uri();

    let request = write_test_file(
        &workdir,
        "request.http",
        "GET ${env(URL)}/1\n"
    ).unwrap();

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("ok")
            .set_delay(Duration::from_millis(1_000)))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .env("URL", &url)
        .arg("--timeout-ms").arg("500")
        .arg(request.to_str())
        .assert()
        .failure()
        .stderr(formatdoc!(
            "GET {uri}/1... Error: error sending request for url ({uri}/1): operation timed out

            Caused by:
                operation timed out
            ",
            uri = url
        ));
}
