extern crate assert_cmd;
extern crate mockito;
extern crate rstest;

use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_test_utils::write_test_file;
use mockito::mock;
use rstest::{fixture, rstest};
use temp_dir::TempDir;

struct TestData {
    pub _workdir: TempDir,
    pub req1: CanonicalizedPathBuf,
    pub req2: CanonicalizedPathBuf,
}

#[fixture]
fn test_data() -> TestData {
    let workdir = TempDir::new().unwrap();

    let req1 = write_test_file(&workdir, "req1", "GET ${env(URL)}/1").unwrap();

    let req2 = write_test_file(&workdir, "req2", "GET ${env(URL)}/2").unwrap();

    TestData {
        _workdir: workdir,
        req1,
        req2,
    }
}

#[rstest]
fn should_stop_execution_on_status_400(test_data: TestData) {
    let url = mockito::server_url();

    let one = mock("GET", "/1")
        .with_status(400)
        .with_body("invalid param")
        .create();
    let two = mock("GET", "/2").expect(0).with_status(200).create();

    let assert = Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", &url)
        .arg(test_data.req1.to_str())
        .arg(test_data.req2.to_str())
        .assert();

    assert.failure().stderr(format!(
        "GET {base}/1... 400 Bad Request\nError: invalid param\n",
        base = &url
    ));

    one.assert();
    two.assert();
}

#[rstest]
fn should_stop_execution_on_connection_issues(test_data: TestData) {
    let url = "http://unreachableurl.foobar";

    let mut cmd = Command::cargo_bin("fhttp").unwrap();
    cmd.env("URL", url);
    cmd.arg(test_data.req1.to_str());
    cmd.arg(test_data.req2.to_str());

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let assert = cmd.assert();

    assert.failure();

    let expectation = format!(
        "GET {base}/1... Error: error sending request for url ({url}/1): error trying to connect:",
        url = &url,
        base = url,
    );

    assert!(stderr.contains(&expectation));
}
