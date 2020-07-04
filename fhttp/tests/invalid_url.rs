extern crate assert_cmd;

use assert_cmd::Command;

#[test]
fn should_handle_invalid_url() {
    use fhttp_core::test_utils::root;

    let base = root().to_str().unwrap().to_owned();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/invalid_url.http")
        .assert();

    assert
        .failure()
        .stderr(format!("calling '{}/resources/it/requests/invalid_url.http'... Invalid URL: 'notAValidUrl'\n", base));
}
