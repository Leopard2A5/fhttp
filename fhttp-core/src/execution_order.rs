use crate::{Request, Config, FhttpError, path_utils};
use crate::Result;
use crate::Profile;
use std::path::PathBuf;
use crate::profiles::Resolve;
use std::ops::Range;
use crate::request::variable_support::VariableSupport;

pub fn plan_request_order(
    initial_requests: Vec<Request>,
    profile: &Profile,
    config: &Config,
) -> Result<Vec<Request>> {
    let mut preprocessor_stack = vec![];
    let mut requests_with_dependencies = vec![];

    for req in &initial_requests {
        for path in get_env_vars_defined_through_requests(&profile, &req) {
            let req = Request::from_file(&path, true)?;
            preprocess_request(
                req,
                &mut requests_with_dependencies,
                &mut preprocessor_stack,
                &config
            )?;
        }
    }

    for req in initial_requests {
        preprocess_request(
            req,
            &mut requests_with_dependencies,
            &mut preprocessor_stack,
            &config
        )?;
    }

    Ok(requests_with_dependencies)
}

fn preprocess_request(
    req: Request,
    mut list: &mut Vec<Request>,
    mut preprocessor_stack: &mut Vec<PathBuf>,
    config: &Config
) -> Result<()> {
    if list.contains(&req) {
        return Ok(());
    }
    if preprocessor_stack.contains(&req.source_path) {
        return Err(FhttpError::new("cyclic dependency detected!"));
    }
    preprocessor_stack.push(req.source_path.clone());

    for dep in req.dependencies() {
        let dep = Request::from_file(&dep, true)?;
        preprocess_request(dep, &mut list, &mut preprocessor_stack, &config)?;
    }

    preprocessor_stack.pop();
    list.push(req);

    Ok(())
}

fn get_env_vars_defined_through_requests(
    profile: &Profile,
    req: &Request
) -> Vec<PathBuf> {
    let vars: Vec<(&str, Range<usize>)> = req.get_env_vars();
    vars.into_iter()
        .map(|(key, _)| {
            let var = profile.get(key, false).unwrap();
            match var {
                Resolve::RequestLookup(path) => Some(path),
                _ => None
            }
        })
        .filter(|it| it.is_some())
        .map(|it| it.unwrap())
        .map(|path| path_utils::get_dependency_path(profile.source_path(), path.to_str().unwrap()))
        .collect()
}
