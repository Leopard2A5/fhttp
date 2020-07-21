use std::ops::Range;

use regex::{Captures, Match, Regex};
use uuid::Uuid;

use crate::{Config, FhttpError, path_utils, Profile, Request, ResponseStore, Result};
use crate::random_numbers::random_int;
use crate::RE_REQUEST;

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
    pub range: Range<usize>,
    pub default: Option<&'a str>,
}

impl VariableSupport for Request {
    fn get_env_vars(&self) -> Vec<EnvVarOccurrence> {
        lazy_static! {
            static ref RE_ENV: Regex = Regex::new(r##"(?m)\$\{env\(([a-zA-Z0-9-_]+)(\s*,\s*"([^"]*)")?\)}"##).unwrap();
        };

        RE_ENV.captures_iter(&self.text)
            .collect::<Vec<Captures>>()
            .into_iter()
            .rev()
            .map(|capture: Captures| {
                let group = capture.get(0).unwrap();
                let key = capture.get(1).unwrap().as_str();
                let default = capture.get(3)
                    .map(|m| m.as_str());
                EnvVarOccurrence {
                    name: key,
                    range: group.start()..group.end(),
                    default,
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
        _replace_request_dependencies(self, &response_store);

        Ok(())
    }
}

fn _replace_env_vars(
    req: &mut Request,
    profile: &Profile,
    config: &Config,
    response_store: &ResponseStore,
) -> Result<()> {
    let variables = req.get_env_vars();

    if !variables.is_empty() {
        let mut buffer = req.text.clone();

        for occurrence in variables {
            let value = profile.get(occurrence.name, config, response_store, occurrence.default)?;
            buffer.replace_range(occurrence.range, &value);
        }
        req.text = buffer;
    }

    Ok(())
}

fn _replace_uuids(req: &mut Request) {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{uuid\(\)}").unwrap();
    };

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&req.text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if !reversed_captures.is_empty() {
        let mut buffer = req.text.clone();

        for capture in reversed_captures {
            let group = capture.get(0).unwrap();
            let range = group.start()..group.end();
            let value = Uuid::new_v4().to_string();

            buffer.replace_range(range, &value);
        }

        req.text = buffer;
    }
}

fn _replace_random_ints(req: &mut Request) -> Result<()> {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{randomInt\(\s*([+-]?\d+)?\s*(,\s*([+-]?\d+)\s*)?\)}").unwrap();
    };

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&req.text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if !reversed_captures.is_empty() {
        let mut buffer = req.text.clone();

        for capture in reversed_captures {
            let group = capture.get(0).unwrap();
            let (min, max) = _parse_min_max(
                capture.get(1),
                capture.get(3)
            )?;

            let range = group.start()..group.end();
            let value = random_int(min, max);

            buffer.replace_range(range, &value.to_string());
        }

        req.text = buffer;
    }

    Ok(())
}

fn _parse_min_max(
    min: Option<Match>,
    max: Option<Match>
) -> Result<(i32, i32)> {
    let ret_min = min
        .map(|m| m.as_str().parse::<i32>())
        .unwrap_or(Ok(0))
        .map_err(|_| FhttpError::new(
            format!("min param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)
        ))?;
    let ret_max = max
        .map(|m| m.as_str().parse::<i32>())
        .unwrap_or(Ok(std::i32::MAX))
        .map_err(|_| FhttpError::new(
            format!("max param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)
        ))?;

    if ret_max < ret_min {
        Err(FhttpError::new("min cannot be greater than max"))
    } else {
        Ok((ret_min, ret_max))
    }
}

fn _replace_request_dependencies(
    req: &mut Request,
    response_store: &ResponseStore
) {
    let reversed_captures = RE_REQUEST.captures_iter(&req.text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

    if !reversed_captures.is_empty() {
        let mut buffer = req.text.clone();

        for capture in reversed_captures {
            let whole_match = capture.get(0).unwrap();
            let range = whole_match.start()..whole_match.end();

            let group = capture.get(1).unwrap();
            let path = path_utils::get_dependency_path(&req.source_path, &group.as_str());

            buffer.replace_range(range, &response_store.get(&path));
        }

        req.text = buffer;
    }
}

#[cfg(test)]
mod replace_variables {
    use std::env;

    use indoc::indoc;

    use super::*;

    #[test]
    fn should_replace_env_vars() -> Result<()> {
        env::set_var("SERVER", "server");
        env::set_var("TOKEN", "token");
        env::set_var("BODY", "body");

        let mut req = Request::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://${env(SERVER)}
                Authorization: ${env(TOKEN)}

                X${env(BODY)}X
            "##)
        );

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
    fn should_handle_env_var_default_values() -> Result<()> {
        env::set_var("BODY", "body");

        let mut req = Request::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET ${env(SRV, "http://localhost:8080")}

                ${env(BODY, "default body")}
            "##)
        );

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

        let mut req = Request::new(
            env::current_dir().unwrap(),
            indoc!(r##"
                GET http://X${uuid()}X
            "##)
        );

        req.replace_variables(
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default(),
            &ResponseStore::new()
        )?;

        assert!(REGEX.is_match(&req.text));

        Ok(())
    }
}
