use regex::{Captures, Regex};
use uuid::Uuid;

use crate::{Config, Profile, RequestDef, ResponseStore, Result};
use crate::evaluation::{Evaluation, BaseEvaluation};
use crate::path_utils::RelativePath;
use crate::random_numbers::{random_int, RandomNumberEval, parse_min_max};

pub trait VariableSupport {
    fn get_env_vars(&self) -> Vec<EnvVarOccurrence>;

    fn replace_variables(
        &mut self,
        profile: &Profile,
        config: &Config,
        response_store: &ResponseStore,
    ) -> Result<()>;
}

#[derive(Debug)]
pub struct EnvVarOccurrence<'a> {
    pub name: &'a str,
    pub default: Option<&'a str>,
    pub base_evaluation: BaseEvaluation,
}

impl <'a> AsRef<BaseEvaluation> for EnvVarOccurrence<'a> {
    fn as_ref(&self) -> &BaseEvaluation {
        &self.base_evaluation
    }
}

impl VariableSupport for RequestDef {
    fn get_env_vars(&self) -> Vec<EnvVarOccurrence> {
        lazy_static! {
            static ref RE_ENV: Regex = Regex::new(r##"(?m)(\\*)\$\{env\(([a-zA-Z0-9-_]+)(\s*,\s*"([^"]*)")?\)}"##).unwrap();
        };

        RE_ENV.captures_iter(&self.text)
            .collect::<Vec<Captures>>()
            .into_iter()
            .rev()
            .map(|capture: Captures| {
                let group = capture.get(0).unwrap();
                let backslashes = capture.get(1).unwrap().range().len();
                let key = capture.get(2).unwrap().as_str();
                let default = capture.get(4)
                    .map(|m| m.as_str());
                EnvVarOccurrence {
                    name: key,
                    default,
                    base_evaluation: BaseEvaluation {
                        range: group.start()..group.end(),
                        backslashes,
                    },
                }
            })
            .collect()
    }

    fn replace_variables(
        &mut self,
        profile: &Profile,
        config: &Config,
        response_store: &ResponseStore,
    ) -> Result<()> {
        _replace_env_vars(self, profile, config, response_store)?;
        _replace_uuids(self);
        _replace_random_ints(self)?;
        _replace_request_dependencies(self, response_store)?;

        Ok(())
    }
}

fn _replace_env_vars(
    req: &mut RequestDef,
    profile: &Profile,
    config: &Config,
    response_store: &ResponseStore,
) -> Result<()> {
    let variables = req.get_env_vars();

    if !variables.is_empty() {
        let mut buffer = req.text.clone();

        for occurrence in variables {
            occurrence.replace(&mut buffer, || {
                profile.get(
                    occurrence.name,
                    config,
                    response_store,
                    occurrence.default,
                    req.dependency,
                )
            })?;
        }
        req.text = buffer;
    }

    Ok(())
}

fn _replace_uuids(req: &mut RequestDef) {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)(\\*)\$\{uuid\(\)}").unwrap();
    };

    let reversed_evaluations: Vec<BaseEvaluation> = RE_ENV.captures_iter(&req.text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let group = capture.get(0).unwrap();
            let range = group.start()..group.end();
            let backslashes = capture.get(1).unwrap().as_str().len();
            BaseEvaluation::new(range, backslashes)
        })
        .collect();

    if !reversed_evaluations.is_empty() {
        let mut buffer = req.text.clone();

        for eval in reversed_evaluations {
            let _ = eval.replace(&mut buffer, || { Ok(Uuid::new_v4().to_string()) });
        }

        req.text = buffer;
    }
}

fn _replace_random_ints(req: &mut RequestDef) -> Result<()> {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)(\\*)\$\{randomInt\(\s*([+-]?\d+)?\s*(,\s*([+-]?\d+)\s*)?\)}").unwrap();
    };

    let reversed_random_nums: Vec<RandomNumberEval> = RE_ENV.captures_iter(&req.text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let group = capture.get(0).unwrap();
            let backslashes = capture.get(1).unwrap().as_str().len();
            let min = capture.get(2).map(|it| it.as_str());
            let max = capture.get(4).map(|it| it.as_str());
            let range = group.range();

            RandomNumberEval::new(min, max, range, backslashes)
        })
        .collect();

    if !reversed_random_nums.is_empty() {
        let mut buffer = req.text.clone();

        for eval in reversed_random_nums {
            eval.replace(&mut buffer, || {
                let (min, max) = parse_min_max(
                    eval.min,
                    eval.max,
                )?;
                Ok(random_int(min, max).to_string())
            })?;
        }

        req.text = buffer;
    }

    Ok(())
}

fn _replace_request_dependencies(
    req: &mut RequestDef,
    response_store: &ResponseStore
) -> Result<()> {
    let reversed_evals = req.request_dependencies()?;

    if !reversed_evals.is_empty() {
        let mut buffer = req.text.clone();

        for eval in reversed_evals {
            eval.replace(&mut buffer, || {
                Ok(response_store.get(&req.get_dependency_path(eval.path)?))
            })?;
        }

        req.text = buffer;
    }

    Ok(())
}

#[cfg(test)]
mod replace_variables {
    use std::env;

    use indoc::indoc;

    use super::*;
    use crate::random_numbers::RANDOM_INT_CALLS;
    use crate::FhttpError;
    use crate::test_utils::root;

    #[test]
    fn should_replace_env_vars() -> Result<()> {
        env::set_var("SERVER", "server");
        env::set_var("TOKEN", "token");
        env::set_var("BODY", "body");

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://${env(SERVER)}
                Authorization: ${env(TOKEN)}

                X${env(BODY)}X
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert_eq!(
            &req.text,
            indoc!(r##"
                GET http://server
                Authorization: token

                XbodyX
            "##)
        );

        Ok(())
    }

    #[test]
    fn should_respect_backslashes_for_escaping_env_vars() -> Result<()> {
        env::set_var("VAR", "X");

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://${env(VAR)}

                \${env(VAR)}
                \\${env(VAR)}
                \\\${env(VAR)}
                \\\\${env(VAR)}
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert_eq!(
            &req.text,
            indoc!(r##"
                GET http://X

                ${env(VAR)}
                \X
                \${env(VAR)}
                \\X
            "##)
        );

        Ok(())
    }

    #[test]
    fn should_handle_env_var_default_values() -> Result<()> {
        env::set_var("BODY", "body");

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET ${env(SRV, "http://localhost:8080")}

                ${env(BODY, "default body")}
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert_eq!(
            &req.text,
            indoc!(r##"
                GET http://localhost:8080

                body
            "##)
        );

        Ok(())
    }

    #[test]
    fn should_replace_uuids() -> Result<()> {
        use regex::Regex;
        lazy_static! {
            static ref REGEX: Regex = Regex::new(r"X[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}X").unwrap();
        };

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://X${uuid()}X
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert!(REGEX.is_match(&req.text));

        Ok(())
    }

    #[test]
    fn should_respect_backslashes_replacing_uuids() -> Result<()> {
        use regex::Regex;

        let pattern = "[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}";
        let uuid = "$\\{uuid\\(\\)\\}";
        let format = format!("{p}\\n\\{u}\\n\\\\{p}\\n\\\\\\{u}\\n\\\\\\\\{p}", p=pattern, u=uuid);
        let regex = Regex::new(&format).unwrap();

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://server

                ${uuid()}
                \${uuid()}
                \\${uuid()}
                \\\${uuid()}
                \\\\${uuid()}
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert!(regex.is_match(&req.text));

        Ok(())
    }

    #[test]
    fn should_replace_random_numbers() -> Result<()> {
        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://server

                ${randomInt()}
                ${randomInt(-5)}
                ${randomInt(-5, 7)}
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        RANDOM_INT_CALLS.with(|calls| {
            assert_eq!(*calls.borrow(), vec![
                (-5, 7),
                (-5, std::i32::MAX),
                (0, std::i32::MAX),
            ]);
        });

        Ok(())
    }

    #[test]
    fn random_numbers_should_validate_params() -> Result<()> {
        let profile = Profile::empty(env::current_dir().unwrap());
        let config = Config::default();
        let response_store = ResponseStore::new();

        assert_eq!(
            RequestDef::new(
                env::current_dir().unwrap(),
                format!("GET ${{randomInt({})}}", std::i32::MIN as i64 - 1)
            )?.replace_variables(&profile, &config, &response_store),
            Err(FhttpError::new(format!("min param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)))
        );

        assert_eq!(
            RequestDef::new(
                env::current_dir().unwrap(),
                format!("${{randomInt(0, {})}}", std::i32::MAX as i64 + 1)
            )?.replace_variables(&profile, &config, &response_store),
            Err(FhttpError::new(format!("max param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)))
        );

        assert_eq!(
            RequestDef::new(
                env::current_dir().unwrap(),
                "${randomInt(3, 2)}"
            )?.replace_variables(&profile, &config, &response_store),
            Err(FhttpError::new("min cannot be greater than max"))
        );

        Ok(())
    }

    #[test]
    fn replace_random_numbers_should_respect_backslashes() -> Result<()> {
        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://server

                ${randomInt()}
                \${randomInt()}
                \\${randomInt()}
                \\\${randomInt()}
                \\\\${randomInt()}
            "##)
        )?;

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert_eq!(
            &req.text,
            indoc!(r##"
                GET http://server

                7
                ${randomInt()}
                \7
                \${randomInt()}
                \\7
            "##)
        );

        Ok(())
    }

    #[test]
    fn should_replace_request_dependencies() -> Result<()> {
        let path = root()
            .join("resources/test/requests/dummy.http");
        let profile = Profile::empty(env::current_dir().unwrap());
        let config = Config::default();
        let response_store = {
            let mut tmp = ResponseStore::new();
            tmp.store(path.clone(), "FOO");
            tmp
        };

        let mut req = RequestDef::new(
            env::current_dir().unwrap(),
            indoc!(r#"
                GET server

                ${request("../resources/test/requests/dummy.http")}
                \${request("../resources/test/requests/dummy.http")}
                \\${request("../resources/test/requests/dummy.http")}
                \\\${request("../resources/test/requests/dummy.http")}
            "#)
        )?;
        req.replace_variables(&profile, &config, &response_store)?;

        assert_eq!(
            req.text,
            indoc!(r#"
                GET server

                FOO
                ${request("../resources/test/requests/dummy.http")}
                \FOO
                \${request("../resources/test/requests/dummy.http")}
            "#)
        );

        Ok(())
    }

}