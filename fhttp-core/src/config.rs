
#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub prompt_missing_env_vars: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt_missing_env_vars: false,
        }
    }
}
