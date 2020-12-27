use crate::parsers::Request;
use crate::path_utils::CanonicalizedPathBuf;

pub struct RequestWrapper {
    pub source_path: CanonicalizedPathBuf,
    pub request: Request,
}
