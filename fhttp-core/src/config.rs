use std::{fmt::Display, time::Duration};

#[derive(Debug, Clone, Copy, Default)]
pub struct Config {
    no_prompt: bool,
    verbose: u8,
    quiet: bool,
    print_paths: bool,
    timeout_ms: Option<u64>,
    curl: bool,
}

impl Config {
    pub fn new(
        no_prompt: bool,
        verbose: u8,
        quiet: bool,
        print_paths: bool,
        timeout_ms: Option<u64>,
        curl: bool,
    ) -> Self {
        Config {
            no_prompt,
            verbose,
            quiet,
            print_paths,
            timeout_ms,
            curl,
        }
    }

    pub fn prompt_missing_env_vars(&self) -> bool {
        !self.no_prompt
    }

    pub fn verbosity(&self) -> u8 {
        match self.quiet {
            true => 0,
            false => self.verbose + 1
        }
    }

    pub fn print_file_paths(&self) -> bool {
        self.print_paths
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_ms.map(|it| Duration::from_millis(it))
    }

    pub fn curl(&self) -> bool {
        self.curl
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
