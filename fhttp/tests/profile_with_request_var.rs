extern crate assert_cmd;
extern crate mockito;

use assert_cmd::Command;
use fhttp_test_utils::write_test_file;
use indoc::{indoc, formatdoc};
use mockito::mock;
use temp_dir::TempDir;

#[test]
fn should_resolve() {
    let workdir = TempDir::new().unwrap();
    let url = mockito::server_url();

    let token = write_test_file(
        &workdir,
        "token.http",
        indoc!(r#"
            POST ${env(URL)}/token
            Content-Type: application/json
            
            {
              "username": "${env(USERNAME)}",
              "password": "${env(PASSWORD)}"
            }
            
            > {%
              json $.token
            %}
        "#)
    ).unwrap();

    let create = write_test_file(
        &workdir,
        "create.http",
        &formatdoc!(r#"
            POST ${{env(URL)}}/resources
            Authorization: Bearer ${{request("{token}")}}
            Content-Type: application/json
            
            {{
              "name": "resource"
            }}
            
            > {{%
              json $.id
            %}}
        "#, token = token.to_str())
    ).unwrap();

    let delete = write_test_file(
        &workdir,
        "delete.http",
        &formatdoc!(r#"
            DELETE ${{env(URL)}}/resources/${{env(ID)}}
            Authorization: Bearer ${{request("{token}")}}
        "#, token = token.to_str()),
    ).unwrap();

    let profile = write_test_file(
        &workdir,
        "profiles.json",
        &formatdoc!(
            r#"{{
                "it": {{
                    "variables": {{
                        "USERNAME": "username_from_profile",
                        "PASSWORD": "password_from_profile",
                        "ID": {{
                            "request": "{create}"
                        }},
                        "UNUSED": {{
                            "request": "{delete}"
                        }}
                    }}
                }}
            }}"#,
            create = create.to_str(),
            delete = delete.to_str(),
        )
    ).unwrap();

    let token_mock = mock("POST", "/token")
        .match_body("{\n  \"username\": \"username_from_profile\",\n  \"password\": \"password_from_profile\"\n}")
        .with_body("{\n  \"token\": \"secret_token\"\n}")
        .create();
    let create_mock = mock("POST", "/resources")
        .expect(1)
        .with_status(201)
        .with_body("{\n  \"id\": \"123456\",\n  \"name\": \"resource\"\n}")
        .create();
    let delete_mock = mock("DELETE", "/resources/123456")
        .expect(1)
        .match_header("authorization", "Bearer secret_token")
        .create();

    let assert = Command::cargo_bin("fhttp").unwrap()
        .env("URL", &url)
        .arg("-f").arg(profile.to_str())
        .arg("-p").arg("it")
        .arg(delete.to_str())
        .assert();

    assert
        .success()
        .stderr(
            formatdoc!(r##"
                POST {base}/token... 200 OK
                POST {base}/resources... 201 Created
                DELETE {base}/resources/123456... 200 OK
            "##, base=url)
        );

    token_mock.assert();
    create_mock.assert();
    delete_mock.assert();
}
