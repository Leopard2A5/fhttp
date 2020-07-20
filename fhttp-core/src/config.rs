use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    prompt_missing_env_vars: bool,
    verbosity: u8,
}

impl Config {
    pub fn new(
        prompt_missing_env_vars: bool,
        verbosity: u8,
    ) -> Self {
        Config {
            prompt_missing_env_vars,
            verbosity,
        }
    }

    pub fn prompt_missing_env_vars(&self) -> bool {
        self.prompt_missing_env_vars
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
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
