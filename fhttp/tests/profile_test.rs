extern crate assert_cmd;
extern crate mockito;

use std::env;

use assert_cmd::Command;
use mockito::mock;

#[test]
fn use_custom_profile_file_through_cli_option() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("-f").arg("../resources/it/profiles.json")
        .arg("-p").arg("it")
        .arg("../resources/it/requests/token.http")
        .assert();

    assert.success();

    token.assert();
}

#[test]
fn use_custom_profile_file_through_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "../resources/it/profiles.json");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("-p").arg("it")
        .arg("../resources/it/requests/token.http")
        .assert();

    assert.success();

    token.assert();
}

#[test]
fn profile_file_cli_should_override_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "../resources/it/profiles2.json");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("-f").arg("../resources/it/profiles.json")
        .arg("-p").arg("it")
        .arg("../resources/it/requests/token.http")
        .assert();

    assert.success();

    token.assert();
}

#[test]
fn profile_through_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "../resources/it/profiles.json");
    env::set_var("FHTTP_PROFILE", "it");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/token.http")
        .assert();

    assert.success();

    token.assert();
}

#[test]
fn profile_through_cli_option_should_precede_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "../resources/it/profiles.json");
    env::set_var("FHTTP_PROFILE", "it");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username2\",\n  \"password\": \"password2\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("-p").arg("it2")
        .arg("../resources/it/requests/token.http")
        .assert();

    assert.success();

    token.assert();
}
