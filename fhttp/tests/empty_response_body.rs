extern crate assert_cmd;
extern crate indoc;
extern crate rstest;
extern crate temp_dir;
extern crate wiremock;

use anyhow::Result;
use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_test_utils::write_test_file;
use indoc::indoc;
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[rstest]
async fn async_test() -> Result<()> {
    let workdir = TempDir::new()?;

    let req = write_test_file(
        &workdir,
        "req.http",
        indoc!(
            "
            POST ${env(URL)}/

            > {%
                json $.data.numbers
            %}
        ")
    )?;

    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .env("URL", mock_server.uri())
        .arg(CanonicalizedPathBuf::new(req).to_str())
        .assert()
        .failure()
        .stderr(format!(
            "POST {}/... Error: failed to parse response body as json\nBody was ''\n\nCaused by:\n    EOF while parsing a value at line 1 column 0\n",
            mock_server.uri()
        ));

    Ok(())
}
