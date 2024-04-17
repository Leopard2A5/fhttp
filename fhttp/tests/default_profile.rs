extern crate assert_cmd;
extern crate indoc;
extern crate mockito;
extern crate rstest;
extern crate serde_json;

use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_test_utils::write_test_file;
use indoc::indoc;
use rstest::{fixture, rstest};
use serde_json::json;
use temp_dir::TempDir;

struct TestData {
    pub _workdir: TempDir,
    pub profile: CanonicalizedPathBuf,
    pub req: CanonicalizedPathBuf,
}

#[fixture]
fn test_data() -> TestData {
    let workdir = TempDir::new().unwrap();

    let profile = write_test_file(
        &workdir,
        "profile.json",
        &serde_json::to_string(&json!({
            "default": {
                "variables": {
                    "A": "default-a",
                    "B": "default-b",
                }
            },
            "test": {
                "variables": {
                    "B": "test-b"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let req = write_test_file(
        &workdir,
        "req.http",
        indoc!(
            "
            POST ${env(URL)}/foo

            A=${env(A)}
            B=${env(B)}
        "
        ),
    )
    .unwrap();

    TestData {
        _workdir: workdir,
        profile,
        req,
    }
}

#[rstest]
fn should_always_load_default_profile(test_data: TestData) {
    let mut server = mockito::Server::new();
    let request = server
        .mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=default-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", server.url())
        .arg("-f")
        .arg(test_data.profile.to_str())
        .arg(test_data.req.to_str())
        .assert()
        .success();

    request.assert();
}

#[rstest]
fn should_override_default_profile_with_specified_one(test_data: TestData) {
    let mut server = mockito::Server::new();
    let request = server
        .mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=test-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", server.url())
        .arg("-f")
        .arg(test_data.profile.to_str())
        .arg("-p")
        .arg("test")
        .arg(test_data.req.to_str())
        .assert()
        .success();

    request.assert();
}
