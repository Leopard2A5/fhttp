extern crate assert_cmd;

use assert_cmd::Command;

#[test]
fn should_handle_invalid_url() {
    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/invalid_url.http")
        .assert();

    assert
        .failure()
        .stderr("DELETE notAValidUrl... Error: Invalid URL: 'notAValidUrl'\n\nCaused by:\n    relative URL without a base\n");
}
