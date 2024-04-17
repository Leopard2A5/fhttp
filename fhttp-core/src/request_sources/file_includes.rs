use std::path::Path;

use anyhow::Result;

pub fn load_file_recursively<P: AsRef<Path>>(path: P) -> Result<String> {
    recursive_file_loader::load_file_recursively(path).map_err(anyhow::Error::new)
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use indoc::indoc;

    use crate::test_utils::root;

    use super::*;

    #[test]
    fn should_load_files_recursively() {
        let result =
            load_file_recursively(root().join("resources/nested_file_includes/normal/start.txt"));

        let expectation = String::from_str(indoc! {r##"
                START
                LEVEL-1
                LEVEL-2
                LEVEL-3
                LEVEL-3
            "##})
        .unwrap();

        assert_ok!(result, expectation);
    }

    #[test]
    fn should_detect_cyclic_dependencies() {
        let one = root().join("resources/nested_file_includes/cyclic_dependency/level-1.txt");
        let three = root().join("resources/nested_file_includes/cyclic_dependency/level-3.txt");
        let result = load_file_recursively(
            root().join("resources/nested_file_includes/cyclic_dependency/start.txt"),
        );

        assert_err!(
            result,
            format!(
                "cyclic dependency detected between '{}' and '{}'",
                three.to_str(),
                one.to_str(),
            )
        );
    }

    #[test]
    fn should_respect_escapes() {
        let result =
            load_file_recursively(root().join("resources/nested_file_includes/escaped/start.txt"));

        let expectation = String::from_str(indoc! {r##"
                START
                LEVEL 1
                ${include("level-1.txt")}
                \LEVEL 1
                \${include("level-1.txt")}
                \\LEVEL 1
                \\${include("level-1.txt")}
                \\\LEVEL 1
            "##})
        .unwrap();

        assert_ok!(result, expectation);
    }

    #[test]
    fn should_include_files_preserving_indentation() -> Result<()> {
        let location = root().join("resources/file_includes_indent/request.yaml");
        let text = load_file_recursively(location)?;

        assert_eq!(
            text,
            indoc! {r#"
                method: POST
                url: http://localhost/foo
                body: |
                  before_include
                  include1
                  	include2
                  	    include3
                  \include3
                  after_include
            "#}
        );

        Ok(())
    }
}
