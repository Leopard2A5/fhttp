use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum Body<'a> {
    Plain(Cow<'a, str>),
    File(PathBuf),
}

impl Body<'_> {
    pub fn plain<'a, S: Into<Cow<'a, str>>>(body: S) -> Body<'a> {
        Body::Plain(body.into())
    }
}
