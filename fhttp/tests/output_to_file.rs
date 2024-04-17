use anyhow::Result;
use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use rstest::rstest;
use temp_dir::TempDir;

#[rstest]
async fn should_output_to_new_file() -> Result<()> {
    let mut server = mockito::Server::new();
    let workdir = TempDir::new()?;

    let req = write_test_file(&workdir, "req.http", "GET ${env(URL)}/foo")?;
    let out = workdir.path().join("output.txt");

    let request = server.mock("GET", "/foo").with_body("OK").create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", server.url())
        .arg("-o")
        .arg(out.to_str().unwrap())
        .arg(req.to_str())
        .assert()
        .success();

    let result = std::fs::read_to_string(out)?;
    assert_eq!(&result, "OK\n");
    request.assert();

    Ok(())
}

#[rstest]
async fn should_output_to_and_overwrite_existing_file() -> Result<()> {
    let mut server = mockito::Server::new();
    let workdir = TempDir::new()?;

    let req = write_test_file(&workdir, "req.http", "GET ${env(URL)}/foo")?;
    let out = workdir.path().join("output.txt");
    std::fs::write(&out, "original content")?;

    let request = server.mock("GET", "/foo").with_body("OK").create();

    Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", server.url())
        .arg("-o")
        .arg(out.to_str().unwrap())
        .arg(req.to_str())
        .assert()
        .success();

    let result = std::fs::read_to_string(&out)?;
    assert_eq!(&result, "OK\n");
    request.assert();

    Ok(())
}
