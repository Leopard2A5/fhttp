use std::path::PathBuf;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .into()
}

#[cfg(test)]
pub fn errmsg<T>(r: crate::errors::Result<T>) -> String {
    match r {
        Ok(_) => panic!("expected an Err!"),
        Err(crate::errors::FhttpError { msg }) => msg,
    }
}
