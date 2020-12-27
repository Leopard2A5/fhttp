use std::collections::HashMap;
use crate::path_utils::CanonicalizedPathBuf;

#[derive(Debug)]
pub struct ResponseStore {
    response_data: HashMap<CanonicalizedPathBuf, String>,
}

impl ResponseStore {
    pub fn new() -> Self {
        ResponseStore { response_data: HashMap::new(), }
    }

    pub fn store<V: Into<String>>(
        &mut self,
        path: CanonicalizedPathBuf,
        value: V
    ) {
        self.response_data.insert(path, value.into());
    }

    /// # Panics
    /// panics when key not found.
    pub fn get(&self, path: &CanonicalizedPathBuf) -> String {
        self.response_data[path].clone()
    }
}
