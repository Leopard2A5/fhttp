use std::process::Command;

use fhttp_core::test_utils::root;

static BIN: &str = "target/debug/fhttp";

#[test]
fn should_handle_invalid_url() {
    let base = root().to_str().unwrap().to_owned();

    let output = Command::new(BIN)
        .args(&[
            "../resources/it/requests/invalid_url.http"
        ])
        .output()
        .expect("failed to execute process");

    let stderr = String::from_utf8(output.stderr)
        .expect("stderr is not utf-8");
    eprintln!("{}", stderr);
    assert_eq!(stderr, format!("calling '{}/resources/it/requests/invalid_url.http'... Invalid URL: 'notAValidUrl'\n", base));
    assert!(!output.status.success());
}
