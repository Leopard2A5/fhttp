use std::path::PathBuf;
use crate::path_utils::{CanonicalizedPathBuf, canonicalize};

pub fn root() -> CanonicalizedPathBuf {
    canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .into()
    ).unwrap()
}

#[cfg(test)]
pub fn errmsg<T>(r: crate::errors::Result<T>) -> String {
    match r {
        Ok(_) => panic!("expected an Err!"),
        Err(crate::errors::FhttpError { msg }) => msg,
    }
}
