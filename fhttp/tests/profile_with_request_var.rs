extern crate mockito;

use std::env;
use std::process::Command;

use fhttp_core::test_utils::root;

use mockito::mock;

static BIN: &str = "target/debug/fhttp";

#[test]
fn should_resolve() {
    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let base = root().to_str().unwrap().to_owned();

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
            "-f", "../resources/it/profiles-request-dependency.json",
            "-p", "it",
            "../resources/it/requests/delete_by_env_var.http"
        ])
        .output()
        .expect("failed to execute process");
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(output.status.success());

    assert_eq!(stderr, format!(r##"calling '{base}/resources/it/requests/token.http'... 200 OK
calling '{base}/resources/it/requests/create.http'... 201 Created
calling '{base}/resources/it/requests/delete_by_env_var.http'... 200 OK
"##, base=base
    ));

    token.assert();
    create.assert();
    delete.assert();
}
