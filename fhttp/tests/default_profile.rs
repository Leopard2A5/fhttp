extern crate assert_cmd;
extern crate mockito;

use std::env;

use assert_cmd::Command;
use mockito::mock;

#[test]
fn should_always_load_default_profile() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=default-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp").unwrap()
        .arg("-f").arg("../resources/default_profile/profiles.json")
        .arg("../resources/default_profile/request.http")
        .assert()
        .success();

    request.assert();
}

#[test]
fn should_override_default_profile_with_specified_one() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=test-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp").unwrap()
        .arg("-f").arg("../resources/default_profile/profiles.json")
        .arg("-p").arg("test")
        .arg("../resources/default_profile/request.http")
        .assert()
        .success();

    request.assert();
}
