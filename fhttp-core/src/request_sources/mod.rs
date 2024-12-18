use std::marker::PhantomData;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::parsers::{parse_gql_str, parse_str};
use crate::path_utils::{canonicalize, CanonicalizedPathBuf, RelativePath};
use crate::preprocessing::dependant::{request_dependencies, Dependant};
use crate::request_sources::request_dependency_eval::RequestDependencyEval;
use crate::request_sources::request_wrapper::RequestWrapper;
use crate::request_sources::structured_request_source::{
    parse_request_from_json, parse_request_from_yaml,
};
use crate::{Config, Profile, ResponseStore};
use anyhow::Result;
use file_includes::load_file_recursively;
use variable_support::replace_evals;

pub mod file_includes;
pub mod request_dependency_eval;
pub mod request_wrapper;
pub mod structured_request_source;
pub mod variable_support;

pub struct Raw;
pub struct Preprocessed;

// #[derive(Debug, Eq)]
pub struct RequestSource<State = Raw> {
    state: PhantomData<State>,
    pub source_path: CanonicalizedPathBuf,
    pub text: String,
    pub dependency: bool,
}

impl<State> RequestSource<State> {
    pub fn from_file<P: AsRef<Path>>(path: P, dependency: bool) -> Result<Self> {
        let path = canonicalize(path.as_ref())?;
        let content = load_file_recursively(&path)?;

        Self::_new(path, content, dependency)
    }

    #[cfg(test)]
    pub fn new<P: Into<PathBuf>, T: Into<String>>(path: P, text: T) -> Result<Self> {
        let path = canonicalize(&path.into())?;
        RequestSource::_new(path, text, false)
    }

    fn _new<S: Into<String>>(
        path: CanonicalizedPathBuf,
        text: S,
        dependency: bool,
    ) -> Result<Self> {
        let ret = RequestSource {
            state: PhantomData,
            source_path: path,
            text: text.into(),
            dependency,
        };

        Ok(ret)
    }

    pub fn unescaped_dependency_paths(&self) -> Result<Vec<CanonicalizedPathBuf>> {
        self.unescaped_dependencies()?
            .into_iter()
            .map(|dep| self.get_dependency_path(dep.path))
            .collect()
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

impl RequestSource {
    pub fn replace_variables(
        self,
        profile: &Profile,
        config: &Config,
        response_store: &ResponseStore,
    ) -> Result<RequestSource<Preprocessed>> {
        let new_text = replace_evals(
            self.text,
            &self.source_path,
            self.dependency,
            profile,
            config,
            response_store,
        )?;

        Ok(RequestSource {
            text: new_text,
            state: PhantomData,
            source_path: self.source_path,
            dependency: self.dependency,
        })
    }
}

impl<T> Dependant for RequestSource<T> {
    fn dependencies(&self) -> Result<Vec<RequestDependencyEval>> {
        request_dependencies(&self.text)
    }
}

impl<T> AsRef<Path> for RequestSource<T> {
    fn as_ref(&self) -> &Path {
        self.source_path.as_ref()
    }
}

impl<T> PartialEq for RequestSource<T> {
    fn eq(&self, other: &Self) -> bool {
        self.source_path == other.source_path
    }
}
