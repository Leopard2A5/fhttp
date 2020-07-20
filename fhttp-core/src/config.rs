use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    prompt_missing_env_vars: bool,
    verbosity: u8,
    quiet: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt_missing_env_vars: false,
            verbosity: 1,
            quiet: false
        }
    }
}

impl Config {
    pub fn new(
        prompt_missing_env_vars: bool,
        verbosity: u8,
        quiet: bool
    ) -> Self {
        Config {
            prompt_missing_env_vars,
            verbosity,
            quiet,
        }
    }

    pub fn prompt_missing_env_vars(&self) -> bool {
        self.prompt_missing_env_vars
    }

    pub fn verbosity(&self) -> u8 {
        match self.quiet {
            true => 0,
            false => self.verbosity
        }
    }

    pub fn log<S: Display>(
        &self,
        level: u8,
        message: S
    ) {
        if self.verbosity() >= level {
            eprint!("{}", message);
        }
    }

    pub fn logln<S: Display>(
        &self,
        level: u8,
        message: S
    ) {
        if self.verbosity() >= level {
            eprintln!("{}", message);
        }
    }
}
