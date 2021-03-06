use crate::{Config, FhttpError, RequestDef};
use crate::path_utils::{RelativePath, CanonicalizedPathBuf};
use crate::Profile;
use crate::request_def::variable_support::{EnvVarOccurrence, VariableSupport};
use crate::Result;

pub fn plan_request_order(
    initial_requests: Vec<RequestDef>,
    profile: &Profile,
    config: &Config,
) -> Result<Vec<RequestDef>> {
    let mut preprocessor_stack = vec![];
    let mut requests_with_dependencies = vec![];

    for req in &initial_requests {
        for path in get_env_vars_defined_through_requests(&profile, &req)? {
            let req = RequestDef::from_file(&path, true)?;
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
    req: RequestDef,
    mut list: &mut Vec<RequestDef>,
    mut preprocessor_stack: &mut Vec<CanonicalizedPathBuf>,
    config: &Config
) -> Result<()> {
    if list.contains(&req) {
        return Ok(());
    }
    if preprocessor_stack.contains(&req.source_path) {
        return Err(FhttpError::new("cyclic dependency detected!"));
    }
    preprocessor_stack.push(req.source_path.clone());

    for dep in req.dependencies()? {
        let dep = RequestDef::from_file(dep, true)?;
        preprocess_request(dep, &mut list, &mut preprocessor_stack, &config)?;
    }

    preprocessor_stack.pop();
    list.push(req);

    Ok(())
}

fn get_env_vars_defined_through_requests(
    profile: &Profile,
    req: &RequestDef
) -> Result<Vec<CanonicalizedPathBuf>> {
    let vars: Vec<EnvVarOccurrence> = req.get_env_vars();
    vars.into_iter()
        .map(|occ| profile.defined_through_request(occ.name))
        .filter(|it| it.is_some())
        .map(|it| it.unwrap())
        .map(|path| profile.get_dependency_path(path.to_str().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::{Config, Profile, RequestDef, ResponseStore, Result};
    use crate::execution_order::plan_request_order;
    use crate::test_utils::root;

    #[test]
    fn should_resolve_nested_dependencies() -> Result<()> {
        let root = root()
            .join("resources/test/requests/nested_dependencies");
        let init_path = root.join("1.http");

        let init_request = RequestDef::from_file(&init_path, false)?;

        let profile = Profile::empty(env::current_dir().unwrap());
        let mut response_store = ResponseStore::new();
        let config = Config::default();

        for i in 2..=5 {
            let path = root.join(format!("{}.http", i));
            response_store.store(path, &format!("{}", i));
        }

        let coll = plan_request_order(vec![init_request], &profile, &config)?
            .into_iter()
            .map(|req| req.source_path)
            .collect::<Vec<_>>();

        let foo = (1..=5).into_iter()
            .rev()
            .map(|i| root.join(format!("{}.http", i)))
            .collect::<Vec<_>>();
        assert_eq!(&coll, &foo);

        Ok(())
    }

    #[test]
    fn should_not_resolve_duplicate_dependencies() -> Result<()> {
        let root = root()
            .join("resources/test/requests/duplicate_dependencies");
        let path1 = root.join("1.http");
        let path2 = root.join("2.http");
        let dep_path = root.join("dependency.http");

        let req1 = RequestDef::from_file(&path1, false)?;
        let req2 = RequestDef::from_file(&path2, false)?;

        let profile = Profile::empty(env::current_dir().unwrap());
        let mut response_store = ResponseStore::new();
        let config = Config::default();

        response_store.store(dep_path.clone(), "");
        let coll = plan_request_order(vec![req1, req2], &profile, &config)?
            .into_iter()
            .map(|req| req.source_path)
            .collect::<Vec<_>>();

        assert_eq!(&coll, &[dep_path, path1, path2]);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn should_panic_on_cyclic_dependency() {
        let root = root()
            .join("resources/test/requests/cyclic_dependencies");
        let path1 = root.join("1.http");
        let req1 = RequestDef::from_file(&path1, false).unwrap();

        plan_request_order(
            vec![req1],
            &Profile::empty(env::current_dir().unwrap()),
            &Config::default()
        ).unwrap();
    }
}
