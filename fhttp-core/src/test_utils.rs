#[cfg(test)] extern crate temp_dir;
use std::path::PathBuf;
use crate::path_utils::{CanonicalizedPathBuf, canonicalize};

#[cfg(test)] use anyhow::Result;
#[cfg(test)] use temp_dir::TempDir;

pub fn root() -> CanonicalizedPathBuf {
    canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
    ).unwrap()
}

#[cfg(test)]
pub fn errmsg<T>(r: anyhow::Result<T>) -> String {
    match r {
        Ok(_) => panic!("expected an Err!"),
        Err(e) => e.to_string(),
    }
}

#[cfg(test)]
macro_rules! assert_err(
    ($code:expr, $expectation:expr)=>{
        assert_eq!(
            crate::test_utils::errmsg(
                $code
            ),
            $expectation
        );
    };
);

#[cfg(test)]
macro_rules! assert_ok(
    ($code:expr, $expectation:expr)=>{
        match $code {
            Ok(value) => assert_eq!(value, $expectation),
            Err(_) => panic!("expected an ok value"),
        };
    };
);

#[cfg(test)]
pub fn write_test_file<S: AsRef<str>>(
    workdir: &TempDir,
    filename: S,
    content: S,
) -> Result<CanonicalizedPathBuf> {
    let file = workdir.child(filename.as_ref());
    std::fs::write(&file, content.as_ref().as_bytes())?;
    canonicalize(&file)
}
