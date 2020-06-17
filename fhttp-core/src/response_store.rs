use std::collections::HashMap;
use std::path::{PathBuf};

#[derive(Debug)]
pub struct ResponseStore {
    response_data: HashMap<PathBuf, String>,
}

impl ResponseStore {
    pub fn new() -> Self {
        ResponseStore { response_data: HashMap::new(), }
    }

    pub fn store<P: Into<PathBuf>, V: Into<String>>(
        &mut self,
        path: P,
        value: V
    ) {
        self.response_data.insert(path.into(), value.into());
    }

    /// # Panics
    /// panics when key not found.
    pub fn get(&self, path: &PathBuf) -> String {
        self.response_data[path].clone()
    }
}
