use std::fmt::Display;
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    prompt_missing_env_vars: bool,
    verbosity: u8,
    quiet: bool,
    print_file_paths: bool,
    timeout: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt_missing_env_vars: false,
            verbosity: 1,
            quiet: false,
            print_file_paths: false,
            timeout: None,
        }
    }
}

impl Config {
    pub fn new(
        prompt_missing_env_vars: bool,
        verbosity: u8,
        quiet: bool,
        print_file_paths: bool,
        timeout: Option<Duration>,
    ) -> Self {
        Config {
            prompt_missing_env_vars,
            verbosity,
            quiet,
            print_file_paths,
            timeout,
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

    pub fn print_file_paths(&self) -> bool {
        self.print_file_paths
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
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
