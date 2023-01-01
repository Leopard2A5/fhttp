use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn canonicalize(p: &Path) -> Result<CanonicalizedPathBuf> {
    fs::canonicalize(&p)
        .with_context(|| format!("error opening file {}", p.to_str().unwrap()))
        .map(CanonicalizedPathBuf)
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Default)]
pub struct CanonicalizedPathBuf(PathBuf);

impl CanonicalizedPathBuf {
    pub fn new<P: Into<PathBuf>>(p: P) -> Self {
        CanonicalizedPathBuf(p.into())
    }

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

impl Display for CanonicalizedPathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

impl AsRef<Path> for CanonicalizedPathBuf {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl From<CanonicalizedPathBuf> for PathBuf {
    fn from(cpb: CanonicalizedPathBuf) -> Self {
        cpb.0
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
        origin_path.join(path)
    } else {
        origin_path.parent().unwrap().join(path)
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
