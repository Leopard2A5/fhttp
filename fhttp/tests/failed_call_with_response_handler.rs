extern crate assert_cmd;
extern crate rstest;
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
async fn test() -> Result<()> {
    let mock_server = MockServer::start().await;

    let workdir = TempDir::new()?;

    let req = write_test_file(
        &workdir,
        "req.http",
        indoc!("
            POST ${env(URL)}/

            > {%
                json $.data.numbers
            %}
        ")
    )?;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", mock_server.uri())
        .arg(CanonicalizedPathBuf::new(req).to_str())
        .assert()
        .failure()
        .stderr(format!(
            "POST {}/... 500 Internal Server Error\nError: no response body\n",
            mock_server.uri()
        ));

    Ok(())
}
