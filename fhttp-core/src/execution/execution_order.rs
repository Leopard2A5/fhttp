use anyhow::{anyhow, Result};

use crate::path_utils::{CanonicalizedPathBuf, RelativePath};
use crate::request_sources::variable_support::{get_env_vars, EnvVarOccurrence};
use crate::Profile;
use crate::RequestSource;

pub fn plan_request_order(
    initial_requests: Vec<RequestSource>,
    profile: &Profile,
) -> Result<Vec<RequestSource>> {
    let mut preprocessor_stack = vec![];
    let mut requests_with_dependencies = vec![];

    for req in &initial_requests {
        for path in get_env_vars_defined_through_requests(profile, req)? {
            let req = RequestSource::from_file(&path, true)?;
            preprocess_request(
                req,
                &mut requests_with_dependencies,
                &mut preprocessor_stack,
            )?;
        }
    }

    for req in initial_requests {
        preprocess_request(
            req,
            &mut requests_with_dependencies,
            &mut preprocessor_stack,
        )?;
    }

    Ok(requests_with_dependencies)
}

fn preprocess_request(
    req: RequestSource,
    list: &mut Vec<RequestSource>,
    preprocessor_stack: &mut Vec<CanonicalizedPathBuf>,
) -> Result<()> {
    if list.contains(&req) {
        return Ok(());
    }
    if preprocessor_stack.contains(&req.source_path) {
        return Err(anyhow!("cyclic dependency detected!"));
    }
    preprocessor_stack.push(req.source_path.clone());

    for dep in req.unescaped_dependency_paths()? {
        let dep = RequestSource::from_file(dep, true)?;
        preprocess_request(dep, list, preprocessor_stack)?;
    }

    preprocessor_stack.pop();
    list.push(req);

    Ok(())
}

fn get_env_vars_defined_through_requests(
    profile: &Profile,
    req: &RequestSource,
) -> Result<Vec<CanonicalizedPathBuf>> {
    let vars: Vec<EnvVarOccurrence> = get_env_vars(&req.text);
    vars.into_iter()
        .flat_map(|occ| profile.defined_through_request(occ.name))
        .map(|path| profile.get_dependency_path(path.to_str().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::env;

    use anyhow::Result;
    use indoc::indoc;
    use temp_dir::TempDir;

    use crate::execution::execution_order::plan_request_order;
    use crate::path_utils::canonicalize;
    use crate::test_utils::write_test_file;
    use crate::{Profile, RequestSource, ResponseStore};

    #[test]
    fn should_resolve_nested_dependencies() -> Result<()> {
        let workdir = TempDir::new()?;
        let r1 = write_test_file(&workdir, "1.http", r#"GET ${request("2.http")}"#)?;
        let r2 = write_test_file(&workdir, "2.http", r#"GET ${request("3.http")}"#)?;
        let r3 = write_test_file(&workdir, "3.http", r#"GET ${request("4.http")}"#)?;
        let r4 = write_test_file(&workdir, "4.http", r#"GET ${request("5.http")}"#)?;
        let r5 = write_test_file(&workdir, "5.http", r#"GET http://localhost"#)?;

        let init_request = RequestSource::from_file(&r1, false)?;

        let profile = Profile::empty(env::current_dir()?);
        let mut response_store = ResponseStore::new();

        let requests = vec![r1, r2, r3, r4, r5];
        requests.iter().enumerate().for_each(|(i, r)| {
            response_store.store(r.clone(), &format!("{}", i));
        });

        let coll = plan_request_order(vec![init_request], &profile)?
            .into_iter()
            .map(|req| req.source_path)
            .collect::<Vec<_>>();

        let foo = requests.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(&coll, &foo);

        Ok(())
    }

    #[test]
    fn should_not_resolve_duplicate_dependencies() -> Result<()> {
        let workdir = TempDir::new()?;
        let r1 = write_test_file(&workdir, "1.http", r#"GET ${request("dependency.http")}"#)?;
        let r2 = write_test_file(&workdir, "2.http", r#"GET ${request("dependency.http")}"#)?;
        let dep = write_test_file(&workdir, "dependency.http", r#"GET http://localhost"#)?;

        let req1 = RequestSource::from_file(&r1, false)?;
        let req2 = RequestSource::from_file(&r2, false)?;

        let profile = Profile::empty(env::current_dir().unwrap());
        let mut response_store = ResponseStore::new();

        response_store.store(dep.clone(), "");
        let coll = plan_request_order(vec![req1, req2], &profile)?
            .into_iter()
            .map(|req| req.source_path)
            .collect::<Vec<_>>();

        assert_eq!(&coll, &[dep, r1, r2]);

        Ok(())
    }

    #[test]
    fn should_not_resolve_escaped_dependencies() -> Result<()> {
        let workdir = TempDir::new()?;
        let r1 = write_test_file(
            &workdir,
            "1.http",
            indoc!(
                r#"
                GET server

                \${request("4.http")}
            "#
            ),
        )?;
        let request = RequestSource::from_file(&r1, false)?;

        let profile = Profile::empty(env::current_dir().unwrap());

        let coll = plan_request_order(vec![request], &profile)?
            .into_iter()
            .map(|req| req.source_path)
            .collect::<Vec<_>>();

        assert_eq!(&coll, &[r1]);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn should_panic_on_cyclic_dependency() {
        let workdir = TempDir::new().unwrap();
        let r1 = &workdir.child("1.http");
        let r2 = &workdir.child("2.http");
        std::fs::File::create(r1).unwrap();
        std::fs::File::create(r2).unwrap();

        let r1 = canonicalize(r1).unwrap();
        let r2 = canonicalize(r2).unwrap();

        std::fs::write(
            &r1,
            format!(r#"GET ${{request("{}")}}"#, &r2.to_str()).as_bytes(),
        )
        .unwrap();
        std::fs::write(
            &r2,
            format!(r#"GET ${{request("{}")}}"#, &r1.to_str()).as_bytes(),
        )
        .unwrap();

        let req1 = RequestSource::from_file(&r1, false).unwrap();

        plan_request_order(vec![req1], &Profile::empty(env::current_dir().unwrap())).unwrap();
    }
}
