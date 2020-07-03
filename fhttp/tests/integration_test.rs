extern crate mockito;

use std::env;
use std::process::Command;

use fhttp_core::test_utils::root;

use mockito::mock;

static BIN: &str = "../target/debug/fhttp";

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

    let output = Command::new(BIN)
        .args(&[
            "../resources/it/requests/create.http",
            "../resources/it/requests/update.http",
            "../resources/it/requests/delete.http"
        ])
        .output()
        .expect("failed to execute process");
    assert!(output.status.success());

    let stderr = String::from_utf8(output.stderr)
        .expect("stderr is not utf-8");

    assert_eq!(stderr, format!(r##"calling '{base}/resources/it/requests/token.http'... 200 OK
calling '{base}/resources/it/requests/create.http'... 201 Created
calling '{base}/resources/it/requests/update.http'... 200 OK
calling '{base}/resources/it/requests/delete.http'... 200 OK
"##, base=base
    ));

    let stdout = String::from_utf8(output.stdout)
        .expect("stdout is not utf-8");
    assert_eq!(&stdout, "123456\n\n\n");

    token.assert();
    create.assert();
    update.assert();
    delete.assert();
}
