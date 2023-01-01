extern crate assert_cmd;
extern crate mockito;
extern crate reqwest;
extern crate async_std;

use anyhow::Result;
use assert_cmd::Command;
use async_std::task::block_on;
use temp_dir::TempDir;
use fhttp_core::path_utils::CanonicalizedPathBuf;

#[test]
fn should_show_error_when_asked_to_output_dependencies() -> Result<()> {
    block_on(test())
}

async fn test() -> Result<()> {
    let workdir = TempDir::new()?;

    let req1 = workdir.child("req1.http");
    std::fs::write(&req1, "GET http://localhost".as_bytes())?;

    let req2 = workdir.child("req2.http");
    std::fs::write(&req2, r#"GET ${request("req1.http")}"#.as_bytes())?;

    let workdir = CanonicalizedPathBuf::new(workdir.path());
    let req1 = workdir.join(req1);
    let req2 = workdir.join(req2);

    Command::cargo_bin("fhttp").unwrap()
        .arg("--curl")
        .arg(req1.to_str())
        .arg(req2.to_str())
        .assert()
        .failure()
        .stderr(format!(
            "Error: {}\nis a dependency of\n{}.\nIf you want me to print the curl snippet for both requests you'll need to do them separately.\n",
            req1,
            req2,
        ));

    Ok(())
}
