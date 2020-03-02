extern crate mockito;

use std::env;
use std::process::Command;

use mockito::mock;

static BIN: &str = "target/debug/fhttp";

#[test]
fn should_resolve() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let token = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}\n")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();
    let create = mock("POST", "/resources")
        .expect(1)
        .with_status(201)
        .with_body("{\n  \"id\": \"123456\",\n  \"name\": \"resource\"\n}")
        .create();
    let delete = mock("DELETE", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .create();

    let output = Command::new(BIN)
        .args(&[
            "-f", "resources/it/profiles-request-dependency.json",
            "-p", "it",
            "resources/it/requests/delete_by_env_var.http"
        ])
        .output()
        .expect("failed to execute process");

    eprintln!("stderr: {}", String::from_utf8(output.stderr).unwrap());

    assert!(output.status.success());
    token.assert();
    create.assert();
    delete.assert();
}
