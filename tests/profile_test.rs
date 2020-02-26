extern crate mockito;

use std::env;
use std::process::Command;

use mockito::mock;

static BIN: &str = "target/debug/fhttp";

#[test]
fn use_custom_profile_file_through_cli_option() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}\n")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let output = Command::new(BIN)
        .args(&[
            "-f", "resources/it/profiles.json",
            "-p", "it",
            "resources/it/requests/token.http"
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    token.assert();
}

#[test]
fn use_custom_profile_file_through_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "resources/it/profiles.json");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}\n")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let output = Command::new(BIN)
        .args(&[
            "-p", "it",
            "resources/it/requests/token.http"
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    token.assert();
}

#[test]
fn profile_file_cli_should_override_env_var() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("FHTTP_PROFILE_FILE", "resources/it/profiles2.json");

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}\n")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let output = Command::new(BIN)
        .args(&[
            "-f", "resources/it/profiles.json",
            "-p", "it",
            "resources/it/requests/token.http"
        ])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    token.assert();
}
