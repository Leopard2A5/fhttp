extern crate assert_cmd;
extern crate mockito;
extern crate serde_json;
extern crate indoc;

use anyhow::Result;
use assert_cmd::Command;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use mockito::mock;
use serde_json::json;
use temp_dir::TempDir;
use indoc::indoc;

#[test]
fn should_always_load_default_profile() -> Result<()> {
    let workdir = TempDir::new()?;

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
        )?
    )?;
    let profile = CanonicalizedPathBuf::new(profile);

    let req = workdir.child("req.http");
    std::fs::write(
        &req,
        indoc!("
            POST ${env(URL)}/foo

            A=${env(A)}
            B=${env(B)}
        ").as_bytes()
    )?;
    let req = CanonicalizedPathBuf::new(req);

    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=default-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", &mockito::server_url())
        .arg("-f")
        .arg(profile.to_str())
        .arg(req.to_str())
        .assert()
        .success();

    request.assert();

    Ok(())
}

#[test]
fn should_override_default_profile_with_specified_one() -> Result<()> {
    let workdir = TempDir::new()?;

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
        )?
    )?;
    let profile = CanonicalizedPathBuf::new(profile);

    let req = workdir.child("req.http");
    std::fs::write(
        &req,
        indoc!("
            POST ${env(URL)}/foo

            A=${env(A)}
            B=${env(B)}
        ").as_bytes()
    )?;
    let req = CanonicalizedPathBuf::new(req);

    let request = mock("POST", "/foo")
        .expect(1)
        .match_body("A=default-a\nB=test-b")
        .with_body("OK")
        .create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", &mockito::server_url())
        .arg("-f")
        .arg(profile.to_str())
        .arg("-p")
        .arg("test")
        .arg(req.to_str())
        .assert()
        .success();

    request.assert();

    Ok(())
}
