extern crate assert_cmd;
extern crate wiremock;
extern crate async_std;

use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use indoc::formatdoc;
use rstest::rstest;
use temp_dir::TempDir;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_string_contains};

#[rstest]
async fn multipart_test() {
    let mock_server = MockServer::start().await;

    let workdir = TempDir::new().unwrap();

    let content1 = write_test_file(
        &workdir,
        "multipart_content_1.txt",
        "this is a file\n"
    ).unwrap();

    let content2 = write_test_file(
        &workdir,
        "multipart_content_2.txt",
        "this is another file\n"
    ).unwrap();

    let multipart = write_test_file(
        &workdir,
        "multipart.http",
        &formatdoc!(
            "
                POST ${{env(URL)}}/

                ${{file(\"file\", \"{file1}\")}}
                ${{file(\"data\", \"{file2}\")}}
            ",
            file1 = content1.to_str(),
            file2 = content2.to_str(),
        )
    ).unwrap();

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_string_contains("Content-Disposition: form-data; name=\"file\"; filename=\"multipart_content_1.txt\""))
        .and(body_string_contains("Content-Type: text/plain"))
        .and(body_string_contains("this is a file"))
        .and(body_string_contains("Content-Disposition: form-data; name=\"data\"; filename=\"multipart_content_2.txt\""))
        .and(body_string_contains("this is another file"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("fhttp").unwrap()
        .env("URL", mock_server.uri())
        .arg(multipart.to_str())
        .assert()
        .success();
}
