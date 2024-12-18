use std::path::Path;

use anyhow::Result;
use regex::Captures;
use uuid::Uuid;

use crate::path_utils::get_dependency_path;
use crate::preprocessing::dependant::request_dependencies;
use crate::preprocessing::evaluation::{BaseEvaluation, Evaluation};
use crate::preprocessing::random_numbers::{parse_min_max, random_int, RandomNumberEval};
use crate::{Config, Profile, ResponseStore};

#[derive(Debug)]
pub struct EnvVarOccurrence<'a> {
    pub name: &'a str,
    pub default: Option<&'a str>,
    pub base_evaluation: BaseEvaluation,
}

impl<'a> AsRef<BaseEvaluation> for EnvVarOccurrence<'a> {
    fn as_ref(&self) -> &BaseEvaluation {
        &self.base_evaluation
    }
}

pub fn replace_evals(
    text: String,
    base_path: impl AsRef<Path>,
    dependency: bool,
    profile: &Profile,
    config: &Config,
    response_store: &ResponseStore,
) -> Result<String> {
    let text = replace_env_vars(text, dependency, profile, config, response_store)?;
    let text = replace_uuids(text);
    let text = replace_random_ints(text)?;
    let text = replace_request_dependencies(text, base_path, response_store)?;
    Ok(text)
}

pub fn get_env_vars(text: &str) -> Vec<EnvVarOccurrence> {
    let re_env = regex!(r##"(?m)(\\*)(\$\{env\(([a-zA-Z0-9-_]+)(\s*,\s*"([^"]*)")?\)})"##);

    re_env
        .captures_iter(&text)
        .collect::<Vec<Captures>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let backslashes = capture.get(1).unwrap().range();
            let group = capture.get(2).unwrap();
            let key = capture.get(3).unwrap().as_str();
            let default = capture.get(5).map(|m| m.as_str());
            EnvVarOccurrence {
                name: key,
                default,
                base_evaluation: BaseEvaluation {
                    range: group.range(),
                    backslashes,
                },
            }
        })
        .collect()
}

fn replace_env_vars(
    text: String,
    dependency: bool,
    profile: &Profile,
    config: &Config,
    response_store: &ResponseStore,
) -> Result<String> {
    let variables = get_env_vars(&text);

    if variables.is_empty() {
        Ok(text)
    } else {
        let mut buffer = text.clone();
        for occurrence in variables {
            occurrence.replace(&mut buffer, || {
                profile.get(
                    occurrence.name,
                    config,
                    response_store,
                    occurrence.default,
                    dependency,
                )
            })?;
        }
        Ok(buffer)
    }
}

fn replace_uuids(text: String) -> String {
    let re_env = regex!(r"(?m)(\\*)(\$\{uuid\(\)})");

    let reversed_evaluations: Vec<BaseEvaluation> = re_env
        .captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let backslashes = capture.get(1).unwrap().range();
            let group = capture.get(2).unwrap();
            BaseEvaluation::new(group.range(), backslashes)
        })
        .collect();

    if reversed_evaluations.is_empty() {
        text
    } else {
        let mut buffer = text.clone();

        for eval in reversed_evaluations {
            let _ = eval.replace(&mut buffer, || Ok(Uuid::new_v4().to_string()));
        }

        buffer
    }
}

fn replace_random_ints(text: String) -> Result<String> {
    let re_env = regex!(r"(?m)(\\*)(\$\{randomInt\(\s*([+-]?\d+)?\s*(,\s*([+-]?\d+)\s*)?\)})");

    let reversed_random_nums: Vec<RandomNumberEval> = re_env
        .captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let backslashes = capture.get(1).unwrap().range();
            let group = capture.get(2).unwrap();
            let min = capture.get(3).map(|it| it.as_str());
            let max = capture.get(5).map(|it| it.as_str());
            let range = group.range();

            RandomNumberEval::new(min, max, range, backslashes)
        })
        .collect();

    if reversed_random_nums.is_empty() {
        Ok(text)
    } else {
        let mut buffer = text.clone();

        for eval in reversed_random_nums {
            eval.replace(&mut buffer, || {
                let (min, max) = parse_min_max(eval.min, eval.max)?;
                Ok(random_int(min, max).to_string())
            })?;
        }

        Ok(buffer)
    }
}

fn replace_request_dependencies(
    text: String,
    base_path: impl AsRef<Path>,
    response_store: &ResponseStore,
) -> Result<String> {
    let reversed_evals = request_dependencies(&text)?;

    if reversed_evals.is_empty() {
        Ok(text)
    } else {
        let mut buffer = text.clone();

        for eval in reversed_evals {
            eval.replace(&mut buffer, || {
                Ok(response_store.get(&get_dependency_path(&base_path, eval.path)?))
            })?;
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod replace_variables {
    use std::env;

    use indoc::indoc;

    use crate::preprocessing::random_numbers::RANDOM_INT_CALLS;
    use crate::test_utils::root;

    use super::*;

    #[test]
    fn should_replace_env_vars() -> Result<()> {
        env::set_var("SERVER", "server");
        env::set_var("TOKEN", "token");
        env::set_var("BODY", "body");

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://${env(SERVER)}
                Authorization: ${env(TOKEN)}

                X${env(BODY)}X
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert_eq!(
            &req.text,
            indoc!(
                r##"
                GET http://server
                Authorization: token

                XbodyX
            "##
            )
        );

        Ok(())
    }

    #[test]
    fn should_respect_backslashes_for_escaping_env_vars() -> Result<()> {
        env::set_var("VAR", "X");

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://${env(VAR)}

                \${env(VAR)}
                \\${env(VAR)}
                \\\${env(VAR)}
                \\\\${env(VAR)}
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert_eq!(
            &req.text,
            indoc!(
                r##"
                GET http://X

                ${env(VAR)}
                \X
                \${env(VAR)}
                \\X
            "##
            )
        );

        Ok(())
    }

    #[test]
    fn should_handle_env_var_default_values() -> Result<()> {
        env::set_var("BODY", "body");

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET ${env(SRV, "http://localhost:8080")}

                ${env(BODY, "default body")}
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert_eq!(
            &req.text,
            indoc!(
                r##"
                GET http://localhost:8080

                body
            "##
            )
        );

        Ok(())
    }

    #[test]
    fn should_replace_uuids() -> Result<()> {
        let regex = regex!(r"X[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}X");

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://X${uuid()}X
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert!(regex.is_match(&req.text));

        Ok(())
    }

    #[test]
    fn should_respect_backslashes_replacing_uuids() -> Result<()> {
        use regex::Regex;

        let pattern = "[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}";
        let uuid = "$\\{uuid\\(\\)\\}";
        let format = format!(
            "{p}\\n\\{u}\\n\\\\{p}\\n\\\\\\{u}\\n\\\\\\\\{p}",
            p = pattern,
            u = uuid
        );
        let regex = Regex::new(&format).unwrap();

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://server

                ${uuid()}
                \${uuid()}
                \\${uuid()}
                \\\${uuid()}
                \\\\${uuid()}
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert!(regex.is_match(&req.text));

        Ok(())
    }

    #[test]
    fn should_replace_random_numbers() -> Result<()> {
        RANDOM_INT_CALLS.with(|cell| {
            cell.borrow_mut().clear();
        });

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://server

                ${randomInt()}
                ${randomInt(-5)}
                ${randomInt(-5, 7)}
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        RANDOM_INT_CALLS.with(|calls| {
            assert_eq!(
                *calls.borrow(),
                vec![(-5, 7), (-5, std::i32::MAX), (0, std::i32::MAX),]
            );
        });

        Ok(())
    }

    #[test]
    fn random_numbers_should_validate_params() -> Result<()> {
        let profile = Profile::empty(env::current_dir().unwrap());
        let config = Config::default();
        let response_store = ResponseStore::new();

        assert_err!(
            RequestSource::new(
                env::current_dir().unwrap(),
                format!("GET ${{randomInt({})}}", std::i32::MIN as i64 - 1)
            )?
            .replace_variables(&profile, &config, &response_store),
            format!(
                "min param out of bounds: {}..{}",
                std::i32::MIN,
                std::i32::MAX
            )
        );

        assert_err!(
            RequestSource::new(
                env::current_dir().unwrap(),
                format!("${{randomInt(0, {})}}", std::i32::MAX as i64 + 1)
            )?
            .replace_variables(&profile, &config, &response_store),
            format!(
                "max param out of bounds: {}..{}",
                std::i32::MIN,
                std::i32::MAX
            )
        );

        assert_err!(
            RequestSource::new(env::current_dir().unwrap(), "${randomInt(3, 2)}")?
                .replace_variables(&profile, &config, &response_store),
            "min cannot be greater than max"
        );

        Ok(())
    }

    #[test]
    fn replace_random_numbers_should_respect_backslashes() -> Result<()> {
        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r##"
                GET http://server

                ${randomInt()}
                \${randomInt()}
                \\${randomInt()}
                \\\${randomInt()}
                \\\\${randomInt()}
            "##
            ),
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new(),
        )?;

        assert_eq!(
            &req.text,
            indoc!(
                r##"
                GET http://server

                7
                ${randomInt()}
                \7
                \${randomInt()}
                \\7
            "##
            )
        );

        Ok(())
    }

    #[test]
    fn should_replace_request_dependencies() -> Result<()> {
        let path = root().join("resources/test/requests/dummy.http");
        let profile = Profile::empty(env::current_dir().unwrap());
        let config = Config::default();
        let response_store = {
            let mut tmp = ResponseStore::new();
            tmp.store(path.clone(), "FOO");
            tmp
        };

        let mut req = RequestSource::new(
            env::current_dir().unwrap(),
            indoc!(
                r#"
                GET server

                ${request("../resources/test/requests/dummy.http")}
                \${request("../resources/test/requests/dummy.http")}
                \\${request("../resources/test/requests/dummy.http")}
                \\\${request("../resources/test/requests/dummy.http")}
            "#
            ),
        )?;
        req.replace_variables(&profile, &config, &response_store)?;

        assert_eq!(
            req.text,
            indoc!(
                r#"
                GET server

                FOO
                ${request("../resources/test/requests/dummy.http")}
                \FOO
                \${request("../resources/test/requests/dummy.http")}
            "#
            )
        );

        Ok(())
    }
}
