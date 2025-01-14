use crate::preprocessing::evaluation::Evaluation;
use crate::request_sources::request_dependency_eval::RequestDependencyEval;
use anyhow::Result;
use regex::Captures;

pub trait Dependant {
    fn dependencies(&self) -> Result<Vec<RequestDependencyEval>>;

    fn unescaped_dependencies(&self) -> Result<Vec<RequestDependencyEval>> {
        Ok(self
            .dependencies()?
            .into_iter()
            .filter(|dep| !dep.is_escaped())
            .collect())
    }
}

pub fn request_dependencies(text: &str) -> Result<Vec<RequestDependencyEval>> {
    let re_request = regex!(r#"(?m)(\\*)(\$\{request\("([^"]+)"\)})"#);

    let deps = re_request
        .captures_iter(text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|capture: Captures| {
            let backslashes = capture.get(1).unwrap().range();
            let group = capture.get(2).unwrap();
            let path = capture.get(3).unwrap().as_str();

            RequestDependencyEval::new(path, group.range(), backslashes)
        })
        .collect::<Vec<_>>();

    Ok(deps)
}
