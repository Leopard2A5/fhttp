#[cfg(test)]
use std::path::PathBuf;
use std::path::Path;

use regex::{Regex, Captures};

use crate::errors::Result;
use crate::file_includes::load_file_recursively;
use crate::parsers::{parse_gql_str, parse_str};
use crate::path_utils::{canonicalize, RelativePath, CanonicalizedPathBuf};
use crate::request_def::request_wrapper::RequestWrapper;
use crate::request_def::request_dependency_eval::RequestDependencyEval;
use crate::evaluation::Evaluation;

pub mod variable_support;
pub mod body;
pub mod request_wrapper;
pub mod request_dependency_eval;

lazy_static!{
    pub static ref RE_REQUEST: Regex = Regex::new(r#"(?m)(\\*)\$\{request\("([^"]+)"\)}"#).unwrap();
}

// #[derive(Debug, Eq)]
pub struct RequestDef {
    pub source_path: CanonicalizedPathBuf,
    pub text: String,
    pub dependency: bool,
}

impl RequestDef {

    pub fn from_file<P: AsRef<Path>>(
        path: P,
        dependency: bool,
    ) -> Result<Self> {
        let path = canonicalize(path.as_ref())?;
        let content = load_file_recursively(&path)?;

        RequestDef::_new(path, content, dependency)
    }

    #[cfg(test)]
    pub fn new<P: Into<PathBuf>, T: Into<String>>(
        path: P,
        text: T
    ) -> Result<Self> {
        let path = canonicalize(&path.into())?;
        RequestDef::_new(path, text, false)
    }

    fn _new<T: Into<String>>(
        path: CanonicalizedPathBuf,
        text: T,
        dependency: bool
    ) -> Result<Self> {
        let ret = RequestDef {
            source_path: path,
            text: text.into(),
            dependency,
        };

        Ok(ret)
    }

    pub fn dependencies(&self) -> Result<Vec<CanonicalizedPathBuf>> {
        self.request_dependencies()?.iter()
            .filter(|dep| !dep.is_escaped())
            .map(|dep| self.get_dependency_path(dep.path))
            .collect()
    }

    pub fn request_dependencies(&self) -> Result<Vec<RequestDependencyEval>> {
        let deps = RE_REQUEST.captures_iter(&self.text)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|capture: Captures| {
                let group = capture.get(0).unwrap();
                let backslashes = capture.get(1).unwrap().as_str().len();
                let path = capture.get(2).unwrap().as_str();

                RequestDependencyEval::new(path, group.range(), backslashes)
            })
            .collect::<Vec<_>>();

        Ok(deps)
    }

    pub fn parse(self) -> Result<RequestWrapper> {
        let request = match self.gql_file() {
            true => parse_gql_str(&self.text)?,
            false => parse_str(&self.source_path, &self.text)?,
        };

        Ok(
            RequestWrapper {
                source_path: self.source_path,
                request,
            }
        )
    }

    fn gql_file(&self) -> bool {
        let filename = self.source_path.file_name();

        filename.ends_with(".gql.http") || filename.ends_with(".graphql.http")
    }
}

impl AsRef<Path> for RequestDef {
    fn as_ref(&self) -> &Path {
        self.source_path.as_ref()
    }
}

impl PartialEq for RequestDef {
    fn eq(
        &self,
        other: &Self
    ) -> bool {
        self.source_path == other.source_path
    }
}