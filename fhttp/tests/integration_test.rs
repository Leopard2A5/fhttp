extern crate assert_cmd;
extern crate mockito;

use std::env;

use assert_cmd::Command;
use mockito::mock;

use fhttp_core::test_utils::root;

#[test]
fn complex_test() {
    let base = root().to_str().unwrap().to_owned();

    let url = &mockito::server_url();
    env::set_var("URL", &url);
    env::set_var("USERNAME", "gordon.shumway");
    env::set_var("PASSWORD", "ilikelucky");

    let token = mock("POST", "/token")
        .expect(1)
        .match_body("{\n  \"username\": \"gordon.shumway\",\n  \"password\": \"ilikelucky\"\n}\n")
        .match_header("content-type", "application/json")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();
    let create = mock("POST", "/resources")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .match_header("content-type", "application/json")
        .match_body("{\n  \"name\": \"resource\"\n}\n")
        .with_status(201)
        .with_body("{\n  \"id\": \"123456\",\n  \"name\": \"resource\"\n}")
        .create();
    let update = mock("PATCH", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .match_header("content-type", "application/json")
        .match_body("{\n  \"name\": \"changed resource\"\n}\n")
        .create();
    let delete = mock("DELETE", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .arg("../resources/it/requests/create.http")
        .arg("../resources/it/requests/update.http")
        .arg("../resources/it/requests/delete.http")
        .assert();
    assert
        .success()
        .stdout("123456\n\n\n")
        .stderr(format!(r##"calling '{base}/resources/it/requests/token.http'... 200 OK
calling '{base}/resources/it/requests/create.http'... 201 Created
calling '{base}/resources/it/requests/update.http'... 200 OK
calling '{base}/resources/it/requests/delete.http'... 200 OK
"##, base=base
    ));

    token.assert();
    create.assert();
    update.assert();
    delete.assert();
}
