use crate::path_utils::CanonicalizedPathBuf;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Body {
    Plain(String),
    Multipart(Vec<MultipartPart>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MultipartPart {
    Text {
        name: String,
        text: String,
        mime_str: Option<String>,
    },
    File {
        name: String,
        file_path: CanonicalizedPathBuf,
        mime_str: Option<String>,
    },
}

#[cfg(test)]
impl Body {
    pub fn plain<S: Into<String>>(body: S) -> Body {
        Body::Plain(body.into())
    }
}
