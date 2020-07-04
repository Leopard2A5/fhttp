extern crate assert_cmd;
extern crate mockito;

use std::env;

use assert_cmd::Command;
use mockito::mock;

use fhttp_core::test_utils::root;

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

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("-f").arg("../resources/it/profiles-request-dependency.json")
        .arg("-p").arg("it")
        .arg("../resources/it/requests/delete_by_env_var.http")
        .assert();

    assert
        .success()
        .stderr(format!(r##"calling '{base}/resources/it/requests/token.http'... 200 OK
calling '{base}/resources/it/requests/create.http'... 201 Created
calling '{base}/resources/it/requests/delete_by_env_var.http'... 200 OK
"##, base=base
        ));

    token.assert();
    create.assert();
    delete.assert();
}
