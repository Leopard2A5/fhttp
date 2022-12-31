use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use regex::Captures;

use crate::parsers::{parse_gql_str, parse_str};
use crate::path_utils::{canonicalize, CanonicalizedPathBuf, RelativePath};
use crate::preprocessing::evaluation::Evaluation;
use crate::request_sources::request_dependency_eval::RequestDependencyEval;
use crate::request_sources::request_wrapper::RequestWrapper;
use crate::request_sources::structured_request_source::{
    parse_request_from_json, parse_request_from_yaml,
};
use anyhow::Result;
use file_includes::load_file_recursively;

pub mod file_includes;
pub mod request_dependency_eval;
pub mod request_wrapper;
pub mod structured_request_source;
pub mod variable_support;

// #[derive(Debug, Eq)]
pub struct RequestSource {
    pub source_path: CanonicalizedPathBuf,
    pub text: String,
    pub dependency: bool,
}

impl RequestSource {
    pub fn from_file<P: AsRef<Path>>(path: P, dependency: bool) -> Result<Self> {
        let path = canonicalize(path.as_ref())?;
        let content = load_file_recursively(&path)?;

        RequestSource::_new(path, content, dependency)
    }

    #[cfg(test)]
    pub fn new<P: Into<PathBuf>, T: Into<String>>(path: P, text: T) -> Result<Self> {
        let path = canonicalize(&path.into())?;
        RequestSource::_new(path, text, false)
    }

    fn _new<T: Into<String>>(
        path: CanonicalizedPathBuf,
        text: T,
        dependency: bool,
    ) -> Result<Self> {
        let ret = RequestSource {
            source_path: path,
            text: text.into(),
            dependency,
        };

        Ok(ret)
    }

    pub fn dependencies(&self) -> Result<Vec<CanonicalizedPathBuf>> {
        self.request_dependencies()?
            .iter()
            .filter(|dep| !dep.is_escaped())
            .map(|dep| self.get_dependency_path(dep.path))
            .collect()
    }

    pub fn request_dependencies(&self) -> Result<Vec<RequestDependencyEval>> {
        let re_request = regex!(r#"(?m)(\\*)(\$\{request\("([^"]+)"\)})"#);

        let deps = re_request
            .captures_iter(&self.text)
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

    pub fn parse(self) -> Result<RequestWrapper> {
        let path = self.source_path.to_str().to_lowercase();
        let request = if path.ends_with(".gql.http") || path.ends_with(".graphql.http") {
            parse_gql_str(&self.text)?
        } else if path.ends_with(".json") {
            parse_request_from_json(&self.source_path, &self.text)?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            parse_request_from_yaml(&self.source_path, &self.text)?
        } else {
            parse_str(&self.source_path, &self.text)?
        };

        Ok(RequestWrapper {
            source_path: self.source_path,
            request,
        })
    }
}

impl AsRef<Path> for RequestSource {
    fn as_ref(&self) -> &Path {
        self.source_path.as_ref()
    }
}

impl PartialEq for RequestSource {
    fn eq(&self, other: &Self) -> bool {
        self.source_path == other.source_path
    }
}
