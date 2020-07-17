
#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub prompt_missing_env_vars: bool,
    pub verbosity: u8,
}

impl Config {
    pub fn print_request_paths_and_status(&self) -> bool {
        self.verbosity >= 1
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt_missing_env_vars: false,
            verbosity: 1,
        }
    }
}
