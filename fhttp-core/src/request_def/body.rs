use crate::path_utils::CanonicalizedPathBuf;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Body {
    Plain(String),
    Files(Vec<File>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct File {
    pub name: String,
    pub path: CanonicalizedPathBuf,
}

#[cfg(test)]
impl Body {
    pub fn plain<S: Into<String>>(body: S) -> Body {
        Body::Plain(body.into())
    }
}
