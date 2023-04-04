extern crate assert_cmd;
extern crate mockito;
extern crate indoc;

use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_test_utils::write_test_file;
use mockito::mock;
use rstest::{fixture, rstest};
use temp_dir::TempDir;
use indoc::indoc;


struct TestData {
    _workdir: TempDir,
    pub profile1: CanonicalizedPathBuf,
    pub profile2: CanonicalizedPathBuf,
    pub token: CanonicalizedPathBuf,
}

#[fixture]
fn test_data() -> TestData {
    let workdir = TempDir::new().unwrap();

    let profile1 = write_test_file(
        &workdir,
        "profiles.json",
        r#"{
            "it": {
              "variables": {
                "USERNAME": "username_from_profile",
                "PASSWORD": "password_from_profile"
              }
            },
            "it2": {
              "variables": {
                "USERNAME": "username2",
                "PASSWORD": "password2"
              }
            }
          }
        "#,
    ).unwrap();

    let profile2 = write_test_file(
        &workdir,
        "profiles2.json",
        r#"{
            "it": {
              "variables": {
                "USERNAME": "wrong_username",
                "PASSWORD": "wrong_password"
              }
            }
          }
        "#,
    ).unwrap();

    let token = write_test_file(
        &workdir,
        "token.http",
        &indoc!(r#"
            POST ${env(URL)}/token
            Content-Type: application/json
            
            {
              "username": "${env(USERNAME)}",
              "password": "${env(PASSWORD)}"
            }
            
            > {%
              json $.token
            %}
        "#)
    ).unwrap();

    TestData {
        _workdir: workdir,
        profile1,
        profile2,
        token,
    }
}

#[rstest]
fn use_custom_profile_file_through_cli_option(test_data: TestData) {
    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", mockito::server_url())
        .arg("-f").arg(test_data.profile1.to_str())
        .arg("-p").arg("it")
        .arg(test_data.token.to_str())
        .assert();

    assert.success();

    token.assert();
}

#[rstest]
fn use_custom_profile_file_through_env_var(test_data: TestData) {
    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", mockito::server_url())
        .env("FHTTP_PROFILE_FILE", test_data.profile1.to_str())
        .arg("-p").arg("it")
        .arg(test_data.token.to_str())
        .assert();

    assert.success();

    token.assert();
}

#[rstest]
fn profile_file_cli_should_override_env_var(test_data: TestData) {
    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", mockito::server_url())
        .env("FHTTP_PROFILE_FILE", test_data.profile2.to_str())
        .arg("-f").arg(test_data.profile1.to_str())
        .arg("-p").arg("it")
        .arg(test_data.token.to_str())
        .assert();

    assert.success();

    token.assert();
}

#[rstest]
fn profile_through_env_var(test_data: TestData) {
    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", mockito::server_url())
        .env("FHTTP_PROFILE_FILE", test_data.profile1.to_str())
        .env("FHTTP_PROFILE", "it")
        .arg(test_data.token.to_str())
        .assert();

    assert.success();

    token.assert();
}

#[rstest]
fn profile_through_cli_option_should_precede_env_var(test_data: TestData) {
    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username2\",\n  \"password\": \"password2\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", mockito::server_url())
        .env("FHTTP_PROFILE", "it")
        .arg("-f").arg(test_data.profile1.to_str())
        .arg("-p").arg("it2")
        .arg(test_data.token.to_str())
        .assert();

    assert.success();

    token.assert();
}
