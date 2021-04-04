use std::path::{Path, PathBuf};
use std::fs;
use crate::{FhttpError, Result};

pub fn canonicalize(p: &Path) -> Result<CanonicalizedPathBuf> {
    fs::canonicalize(&p)
        .map_err(|e| FhttpError::new(format!("error opening file {}: {:?}", p.to_str().unwrap(), e.kind())))
        .map(|p| CanonicalizedPathBuf(p))
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Default)]
pub struct CanonicalizedPathBuf(PathBuf);

impl CanonicalizedPathBuf {
    pub fn to_str(&self) -> &str {
        self.0.to_str().expect("encountered a non-utf8 character in file path!")
    }

    /// This function may panic, it's intended for test purposes!
    pub fn join<P: AsRef<Path>>(
        &self,
        path: P
    ) -> Self {
        canonicalize(&self.0.join(path)).unwrap()
    }

    #[cfg(test)]
    pub fn path_buf(self) -> PathBuf {
        self.0
    }

    pub fn file_name(&self) -> &str {
        self.0.file_name().unwrap()
            .to_str().expect("encountered a non-utf8 character in file path!")
    }
}

impl AsRef<Path> for CanonicalizedPathBuf {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

fn get_dependency_path<O: AsRef<Path>>(
    origin_path: O,
    path: &str
) -> Result<CanonicalizedPathBuf> {
    let origin_path = origin_path.as_ref();
    let path = Path::new(path);
    let ret = if path.is_absolute() {
        path.to_path_buf()
    } else if origin_path.is_dir() {
        origin_path.join(path).to_path_buf()
    } else {
        origin_path.parent().unwrap().join(path).to_path_buf()
    };

    canonicalize(&ret)
}

pub trait RelativePath {
    fn get_dependency_path(&self, path: &str) -> Result<CanonicalizedPathBuf>;
}

impl <T: AsRef<Path>> RelativePath for T {
    fn get_dependency_path(&self, path: &str) -> Result<CanonicalizedPathBuf> {
        get_dependency_path(&self.as_ref(), path)
    }
}
