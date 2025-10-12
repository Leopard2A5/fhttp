use assert_cmd::Command;
use rstest::rstest;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[rstest]
async fn test_stdin() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp")?
        .env("URL", mock_server.uri())
        .arg("-")
        .write_stdin(format!("GET {}/", mock_server.uri()))
        .assert()
        .stderr(format!("GET {}/... 200 OK\n", mock_server.uri()))
        .stdout("ok\n");

    Ok(())
}

#[rstest]
async fn should_not_allow_mixing_stdin_and_file_mode() -> anyhow::Result<()> {
    Command::cargo_bin("fhttp")?
        .arg("-")
        .arg("non-existent file")
        .write_stdin("GET http://localhost/foo/")
        .assert()
        .stderr("Error: Cannot mix reading from stdin (-) and file paths.");

    Ok(())
}
