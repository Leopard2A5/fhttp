use anyhow::Result;
use fhttp_core::path_utils::{CanonicalizedPathBuf, canonicalize};
use temp_dir::TempDir;

pub fn write_test_file<S: AsRef<str>>(
    workdir: &TempDir,
    filename: S,
    content: S,
) -> Result<CanonicalizedPathBuf> {
    let file = workdir.child(filename.as_ref());
    std::fs::write(&file, content.as_ref().as_bytes())?;
    canonicalize(&file)
}
