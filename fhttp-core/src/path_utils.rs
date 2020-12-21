use std::path::{Path, PathBuf};

fn get_dependency_path(
    origin_path: &Path,
    path: &str
) -> PathBuf {
    let path = Path::new(path);
    let ret = if path.is_absolute() {
        path.to_path_buf()
    } else if origin_path.is_dir() {
        origin_path.join(path).to_path_buf()
    } else {
        origin_path.parent().unwrap().join(path).to_path_buf()
    };

    std::fs::canonicalize(&ret).unwrap()
}

pub trait RelativePath {
    fn get_dependency_path(&self, path: &str) -> PathBuf;
}

impl <T: AsRef<Path>> RelativePath for T {
    fn get_dependency_path(&self, path: &str) -> PathBuf {
        get_dependency_path(&self.as_ref(), path)
    }
}
