use clap::{ArgAction, Parser};
use fhttp_core::Config;

#[derive(Parser, Debug, Clone, Default)]
#[command(author, version, about)]
pub struct Args {
    #[arg(required = true, help = "the request files to execute")]
    pub files: Vec<String>,

    #[arg(
        long,
        help = "fail the program instead of prompting for missing environment variables"
    )]
    pub no_prompt: bool,

    #[arg(
        short,
        long,
        env = "FHTTP_PROFILE",
        help = "profile to use. can be set by env var FHTTP_PROFILE"
    )]
    pub profile: Option<String>,

    #[arg(
        short = 'f',
        long,
        env = "FHTTP_PROFILE_FILE",
        help = "profile file to use. defaults to fhttp-config.json. can be set by env var FHTTP_PROFILE_FILE"
    )]
    pub profile_file: Option<String>,

    #[arg(short, long, action = ArgAction::Count, help = "sets the level of verbosity")]
    pub verbose: u8,

    #[arg(short, long, help = "suppress log outputs")]
    pub quiet: bool,

    #[arg(
        short = 'P',
        long,
        help = "print request file paths instead of method and url"
    )]
    pub print_paths: bool,

    #[arg(short, long, help = "time out after this many ms on each request")]
    pub timeout_ms: Option<u64>,

    #[arg(
        short,
        long,
        help = "print curl commands instead of executing given requests. Dependencies are still executed"
    )]
    pub curl: bool,

    #[arg(short, long, help = "redirect output to the specified file")]
    pub out: Option<String>,
}

impl From<Args> for Config {
    fn from(val: Args) -> Self {
        Config::new(
            val.no_prompt,
            val.verbose,
            val.quiet,
            val.print_paths,
            val.timeout_ms,
            val.curl,
        )
    }
}
