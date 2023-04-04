use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use regex::{Captures, Match};

use crate::path_utils::{canonicalize, CanonicalizedPathBuf, RelativePath};
use crate::preprocessing::evaluation::{BaseEvaluation, Evaluation};

pub fn load_file_recursively<P: AsRef<Path>>(path: P) -> Result<String> {
    RecursiveFileLoader::new().load_file_recursively(&canonicalize(path.as_ref())?)
}

struct RecursiveFileLoader {
    resolved_paths: RefCell<HashMap<CanonicalizedPathBuf, String>>,
    resolution_stack: RefCell<Vec<CanonicalizedPathBuf>>,
}

impl RecursiveFileLoader {
    fn new() -> Self {
        RecursiveFileLoader {
            resolved_paths: RefCell::new(HashMap::new()),
            resolution_stack: RefCell::new(Vec::new()),
        }
    }

    fn load_file_recursively(&self, path: &CanonicalizedPathBuf) -> Result<String> {
        self.get_text_for_path(path)
    }

    fn find_includes(
        &self,
        source_path: &CanonicalizedPathBuf,
        text: &str,
    ) -> Result<Vec<Include>> {
        let re_env = regex!(r##"(?m)(^\s*)?(\\*)(\$\{include(_indent)?\("([^"]*)"\)})"##);

        let reversed_captures: Result<Vec<Include>> = re_env
            .captures_iter(text)
            .collect::<Vec<Captures>>()
            .into_iter()
            .rev()
            .map(|capture| {
                let backslashes = capture.get(2).unwrap().range();
                let expression: Match = capture.get(3).unwrap();
                let indent: Option<Match> = capture.get(4);
                let indentation = capture.get(1).map(|it| String::from(it.as_str())).unwrap();
                let path = capture.get(5).unwrap().as_str();
                let path = source_path.get_dependency_path(path)?;

                let indentation = indent.map(|_| indentation);

                Ok(Include::new(
                    expression.range(),
                    path,
                    backslashes,
                    indentation,
                ))
            })
            .collect();

        reversed_captures
    }

    fn get_text_for_path(&self, path: &CanonicalizedPathBuf) -> Result<String> {
        if let Some(content) = self.resolved_paths.borrow().get(path) {
            return Ok(content.clone());
        }

        if self.resolution_stack.borrow().contains(path) {
            let stack = self.resolution_stack.borrow();
            let last = stack.last().unwrap().to_str();
            return Err(anyhow!(
                "cyclic dependency detected between '{}' and '{}'",
                last,
                path.to_str(),
            ));
        } else {
            self.resolution_stack.borrow_mut().push(path.clone());
        }

        let mut content = fs::read_to_string(path)
            .with_context(|| format!("error reading file {}", path.to_str()))?;

        let includes = self.find_includes(path, &content)?;
        for include in includes {
            include.replace(&mut content, || {
                let text = self.get_text_for_path(&include.path)?;
                let text = match &include.indentation {
                    None => text,
                    Some(indentation) => text
                        .lines()
                        .enumerate()
                        .map(|(index, line)| match index {
                            0 => line.to_string(),
                            _ => {
                                let mut tmp = indentation.clone();
                                tmp += line;
                                tmp
                            }
                        })
                        .join("\n"),
                };

                Ok(match text.chars().last() {
                    Some('\n') => text[0..(text.len() - 1)].to_owned(),
                    _ => text,
                })
            })?;
        }

        self.resolved_paths
            .borrow_mut()
            .insert(path.clone(), content.clone());
        self.resolution_stack.borrow_mut().pop();

        Ok(content)
    }
}

#[derive(Debug)]
struct Include {
    path: CanonicalizedPathBuf,
    base_eval: BaseEvaluation,
    indentation: Option<String>,
}

impl Include {
    pub fn new(
        range: Range<usize>,
        path: CanonicalizedPathBuf,
        backslashes: Range<usize>,
        indentation: Option<String>,
    ) -> Self {
        Include {
            path,
            base_eval: BaseEvaluation { range, backslashes },
            indentation,
        }
    }
}

impl AsRef<BaseEvaluation> for Include {
    fn as_ref(&self) -> &BaseEvaluation {
        &self.base_eval
    }
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
            load_file_recursively(&root().join("resources/nested_file_includes/normal/start.txt"));

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
            &root().join("resources/nested_file_includes/cyclic_dependency/start.txt"),
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
            load_file_recursively(&root().join("resources/nested_file_includes/escaped/start.txt"));

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
        let text = load_file_recursively(&location)?;

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
