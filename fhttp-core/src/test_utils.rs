use std::path::PathBuf;
use crate::path_utils::{CanonicalizedPathBuf, canonicalize};

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
