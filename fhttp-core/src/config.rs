use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub prompt_missing_env_vars: bool,
    pub verbosity: u8,
}

impl Config {
    pub fn print_request_paths_and_status(&self) -> bool {
        self.verbosity >= 1
    }

    pub fn print_secret_lookups(&self) -> bool {
        self.verbosity >= 2
    }

    pub fn log<S: Display>(
        &self,
        level: u8,
        message: S
    ) {
        if self.verbosity >= level {
            eprint!("{}", message);
        }
    }

    pub fn logln<S: Display>(
        &self,
        level: u8,
        message: S
    ) {
        if self.verbosity >= level {
            eprintln!("{}", message);
        }
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
