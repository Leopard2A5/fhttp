use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum Body<'a> {
    Plain(Cow<'a, str>),
    Files(Vec<File>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct File {
    name: String,
    path: PathBuf,
}

impl Body<'_> {
    #[cfg(test)]
    pub fn plain<'a, S: Into<Cow<'a, str>>>(body: S) -> Body<'a> {
        Body::Plain(body.into())
    }
}
