extern crate assert_cmd;

use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use temp_dir::TempDir;

#[test]
fn should_handle_invalid_url() {
    let workdir = TempDir::new().unwrap();

    let invalid_url = write_test_file(
        &workdir,
        "invalid_url.http",
        "DELETE notAValidUrl"
    ).unwrap();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg(invalid_url.to_str())
        .assert();

    assert
        .failure()
        .stderr("DELETE notAValidUrl... Error: Invalid URL: 'notAValidUrl'\n\nCaused by:\n    relative URL without a base\n");
}
