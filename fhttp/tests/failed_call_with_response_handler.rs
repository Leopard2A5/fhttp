extern crate assert_cmd;
extern crate async_std;
extern crate rstest;
extern crate wiremock;

use anyhow::Result;
use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use indoc::indoc;
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[rstest]
async fn test() -> Result<()> {
    let mock_server = MockServer::start().await;

    let workdir = TempDir::new()?;

    let req = workdir.child("req.http");
    std::fs::write(
        &req,
        indoc!("
            POST ${env(URL)}/

            > {%
                json $.data.numbers
            %}
        ").as_bytes(),
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
