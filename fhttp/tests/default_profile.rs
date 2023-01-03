extern crate assert_cmd;
extern crate mockito;
extern crate serde_json;
extern crate indoc;
extern crate rstest;

use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use mockito::mock;
use serde_json::json;
use temp_dir::TempDir;
use indoc::indoc;
use rstest::{rstest, fixture};

struct TestData {
    pub _workdir: TempDir,
    pub profile: CanonicalizedPathBuf,
    pub req: CanonicalizedPathBuf,
}

#[fixture]
fn test_data() -> TestData {
    let workdir = TempDir::new().unwrap();

    let profile = workdir.child("profiles.json");
    std::fs::write(
        &profile, 
        serde_json::to_string(
            &json!({
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
            })
        ).unwrap()
    ).unwrap();

    let req = workdir.child("req.http");
    std::fs::write(
        &req,
        indoc!("
            POST ${env(URL)}/foo

            A=${env(A)}
            B=${env(B)}
        ").as_bytes()
    ).unwrap();

    TestData {
        _workdir: workdir,
        profile: CanonicalizedPathBuf::new(profile),
        req: CanonicalizedPathBuf::new(req),
    }
}

#[rstest]
fn should_always_load_default_profile(test_data: TestData) {
    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=default-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", &mockito::server_url())
        .arg("-f")
        .arg(test_data.profile.to_str())
        .arg(test_data.req.to_str())
        .assert()
        .success();

    request.assert();
}

#[rstest]
fn should_override_default_profile_with_specified_one(test_data: TestData) {
    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=test-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", &mockito::server_url())
        .arg("-f")
        .arg(test_data.profile.to_str())
        .arg("-p")
        .arg("test")
        .arg(test_data.req.to_str())
        .assert()
        .success();

    request.assert();
}
