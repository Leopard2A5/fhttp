use assert_cmd::Command;
use indoc::indoc;
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;
use fhttp_test_utils::write_test_file;

const SCRIPT: &str = r#"POST ${env(URL)}/

> {% rhai
    if status > 299 {
        throw `Request failed with status ${status} and body '${body}'`;
    }

    // parse the body as json
    let parsed_body = parse_json(body);

    // process data
    let tmp = parsed_body.data.numbers[1] * 2;

    // convert to string
    tmp.to_string()
%}
"#;

const RESPONSE: &str = r#"{ "data": { "numbers": [11, 42, 13] } }"#;

#[rstest]
async fn test_success() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let workdir = TempDir::new()?;

    let req = write_test_file(
        &workdir,
        "req.http",
        SCRIPT,
    )?;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(RESPONSE))
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp")?
        .env("URL", mock_server.uri())
        .arg(req.to_str())
        .assert()
        .stderr(format!(
            "POST {}/... 200 OK\n",
            mock_server.uri()
        ))
        .stdout("84\n");

    Ok(())
}

#[rstest]
async fn test_failure() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let workdir = TempDir::new()?;

    let req = write_test_file(
        &workdir,
        "req.http",
        SCRIPT,
    )?;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp")?
        .env("URL", mock_server.uri())
        .arg(req.to_str())
        .assert()
        .failure()
        .stderr(format!(
            "POST {}/... Error: Runtime error: Request failed with status 500 and body 'Internal server error' (line 2, position 9)\n",
            mock_server.uri()
        ));

    Ok(())
}