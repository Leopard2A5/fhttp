extern crate mockito;

use std::env;
use std::process::Command;

use fhttp_core::test_utils::root;

use mockito::mock;

static BIN: &str = "target/debug/fhttp";

#[test]
fn should_stop_execution_on_status_400() {
    let base = root().to_str().unwrap().to_owned();

    let url = &mockito::server_url();
    env::set_var("URL", &url);

    let one = mock("GET", "/1")
        .with_status(400)
        .with_body("invalid param")
        .create();
    let two = mock("GET", "/2")
        .expect(0)
        .with_status(200)
        .create();

    let output = Command::new(BIN)
        .args(&[
            "../resources/it/requests/1.http",
            "../resources/it/requests/2.http"
        ])
        .output()
        .expect("failed to execute process");
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert_eq!(stderr, format!(
        "calling '{base}/resources/it/requests/1.http'... 400 Bad Request\ninvalid param\n",
        base=base
    ));
    assert_eq!(output.status.success(), false);

    one.assert();
    two.assert();
}

#[test]
fn should_stop_execution_on_connection_issues() {
    let base = root().to_str().unwrap().to_owned();
    env::set_var("URL", "http://unreachableurl.foobar");

    let output = Command::new(BIN)
        .args(&[
            "../resources/it/requests/1.http",
            "../resources/it/requests/2.http"
        ])
        .output()
        .expect("failed to execute process");
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        stderr.starts_with(
            &format!(
                "calling '{base}/resources/it/requests/1.http'... error sending request for url (http://unreachableurl.foobar/1): error trying to connect:",
                base=base
            )
        )
    );
    assert_eq!(stderr.lines().collect::<Vec<_>>().len(), 1);
    assert_eq!(output.status.success(), false);
}
