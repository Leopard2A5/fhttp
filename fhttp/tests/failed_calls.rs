extern crate assert_cmd;
extern crate mockito;

use std::env;

use assert_cmd::Command;
use mockito::mock;

use fhttp_core::test_utils::root;

#[test]
fn should_stop_execution_on_status_400() {
    let base = root().to_str().unwrap().to_owned();

    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let one = mock("GET", "/1")
        .with_status(400)
        .with_body("invalid param")
        .create();
    let two = mock("GET", "/2")
        .expect(0)
        .with_status(200)
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/1.http")
        .arg("../resources/it/requests/2.http")
        .assert();

    assert
        .failure()
        .stderr(format!(
            "calling '{base}/resources/it/requests/1.http'... 400 Bad Request\ninvalid param\n",
            base=base
        ));

    one.assert();
    two.assert();
}

#[test]
fn should_stop_execution_on_connection_issues() {
    let base = root().to_str().unwrap().to_owned();
    env::set_var("URL", "http://unreachableurl.foobar");

    let mut cmd = Command::cargo_bin("fhttp").unwrap();
    cmd.arg("../resources/it/requests/1.http");
    cmd.arg("../resources/it/requests/2.http");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let assert = cmd.assert();

    assert.failure();

    let expectation = format!(
        "calling '{base}/resources/it/requests/1.http'... error sending request for url (http://unreachableurl.foobar/1): error trying to connect:",
        base=base
    );

    assert!(stderr.contains(&expectation));
}
