extern crate assert_cmd;
extern crate mockito;
extern crate temp_dir;

use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use indoc::{formatdoc, indoc};
use mockito::mock;
use temp_dir::TempDir;

#[test]
fn complex_test() {
    let url = &mockito::server_url();

    let workdir = TempDir::new().unwrap();

    let token = write_test_file(
        &workdir,
        "token",
        indoc!(
            r#"
            POST ${env(URL)}/token
            Content-Type: application/json
            
            {
              "username": "${env(USERNAME)}",
              "password": "${env(PASSWORD)}"
            }
            
            > {%
              json $.token
            %}
        "#
        ),
    )
    .unwrap();

    let create = write_test_file(
        &workdir,
        "create",
        &formatdoc!(
            "
            POST ${{env(URL)}}/resources
            Authorization: Bearer ${{request(\"{}\")}}
            Content-Type: application/json
            
            {{
              \"name\": \"resource\"
            }}
            
            > {{%
              json $.id
            %}}
            ",
            token.to_str(),
        ),
    )
    .unwrap();

    let update = write_test_file(
        &workdir,
        "update",
        &formatdoc!(
            "
            PATCH ${{env(URL)}}/resources/${{request(\"{create}\")}}
            Authorization: Bearer ${{request(\"{token}\")}}
            Content-Type: application/json
            
            {{
              \"name\": \"changed resource\"
            }}
            ",
            create = create.to_str(),
            token = token.to_str(),
        ),
    )
    .unwrap();

    let delete = write_test_file(
        &workdir,
        "delete",
        &formatdoc!(
            "
            DELETE ${{env(URL)}}/resources/${{request(\"{create}\")}}
            Authorization: Bearer ${{request(\"{token}\")}}
            ",
            create = create.to_str(),
            token = token.to_str(),
        ),
    )
    .unwrap();

    let token_mock = mock("POST", "/token")
        .expect(1)
        .match_body("{\n  \"username\": \"gordon.shumway\",\n  \"password\": \"ilikelucky\"\n}")
        .match_header("content-type", "application/json")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();
    let create_mock = mock("POST", "/resources")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .match_header("content-type", "application/json")
        .match_body("{\n  \"name\": \"resource\"\n}")
        .with_status(201)
        .with_body("{\n  \"id\": \"123456\",\n  \"name\": \"resource\"\n}")
        .create();
    let update_mock = mock("PATCH", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .match_header("content-type", "application/json")
        .match_body("{\n  \"name\": \"changed resource\"\n}")
        .create();
    let delete_mock = mock("DELETE", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .create();

    let assert = Command::cargo_bin("fhttp")
        .unwrap()
        .env("URL", url)
        .env("USERNAME", "gordon.shumway")
        .env("PASSWORD", "ilikelucky")
        .arg(create.to_str())
        .arg(update.to_str())
        .arg(delete.to_str())
        .assert();
    assert.success().stdout("123456\n\n\n").stderr(format!(
        r##"POST {base}/token... 200 OK
POST {base}/resources... 201 Created
PATCH {base}/resources/123456... 200 OK
DELETE {base}/resources/123456... 200 OK
"##,
        base = url
    ));

    token_mock.assert();
    create_mock.assert();
    update_mock.assert();
    delete_mock.assert();
}
