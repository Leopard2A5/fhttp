use std::ops::Range;

use regex::{Captures, Regex, Match};

use crate::{Config, path_utils, Profile, Request, Resolve, ResponseStore, Result, FhttpError};
use crate::random_numbers::random_int;
use crate::RE_REQUEST;
use uuid::Uuid;

pub trait VariableSupport {
    fn get_env_vars(&self) -> Vec<(&str, Range<usize>)>;

    fn replace_variables(
        &mut self,
        profile: &Profile,
        config: &Config,
        response_store: &ResponseStore,
    ) -> Result<()>;
}

impl VariableSupport for Request {
    fn get_env_vars(&self) -> Vec<(&str, Range<usize>)> {
        lazy_static! {
            static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{env\(([^}]+)\)}").unwrap();
        };

        RE_ENV.captures_iter(&self.text)
            .collect::<Vec<Captures>>()
            .into_iter()
            .rev()
            .map(|capture| {
                let group = capture.get(0).unwrap();
                let key = capture.get(1).unwrap().as_str();
                (key, group.start()..group.end())
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

        for (key, range) in variables {
            let value = match profile.get(key, config.prompt_missing_env_vars)? {
                Resolve::StringValue(value) => value,
                Resolve::RequestLookup(path) => {
                    let path = path_utils::get_dependency_path(profile.source_path(), path.to_str().unwrap());
                    response_store.get(&path)
                },
            };

            buffer.replace_range(range, &value);
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
