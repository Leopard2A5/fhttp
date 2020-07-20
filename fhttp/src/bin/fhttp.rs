use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::env;

use clap::{App, Arg, crate_authors, crate_version, Values};

use fhttp_core::{Config, Request, Result, FhttpError, Profiles, Profile};
use fhttp_core::Requestpreprocessor;
use fhttp_core::Client;

fn main() {
    let matches = App::new("fhttp")
        .version(crate_version!())
        .author(crate_authors!())
        .about("file-based http client")
        .arg(Arg::with_name("files")
            .multiple(true)
            .required(true)
            .min_values(1)
            .value_name("FILES")
            .help("the request files to execute"))
        .arg(Arg::with_name("no-prompt")
            .long("no-prompt")
            .help("don't prompt for missing environment variables"))
        .arg(Arg::with_name("profile")
            .long("profile")
            .short("p")
            .takes_value(true)
            .help("profile to use. can be overridden by env var FHTTP_PROFILE"))
        .arg(Arg::with_name("profile-file")
            .long("profile-file")
            .short("f")
            .takes_value(true)
            .help("profile file to use. can be overridden by env var FHTTP_PROFILE_FILE. defaults to fhttp-config.json"))
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("sets the level of verbosity"))
        .arg(Arg::with_name("quiet")
            .short("q")
            .long("quiet")
            .help("suppress log outputs")
            .conflicts_with("v"))
        .get_matches();

    let config = Config::new(
        !matches.is_present("no-prompt"),
        matches.occurrences_of("v") as u8 + 1,
        matches.is_present("quiet")
    );

    let profile_path = matches.value_of("profile-file")
        .map(str::to_owned)
        .or_else(|| {
            match env::var("FHTTP_PROFILE_FILE") {
                Ok(path) => Some(path),
                Err(_) => None,
            }
        });

    let profile_name = matches.value_of("profile")
        .map(str::to_owned)
        .or_else(|| {
            match env::var("FHTTP_PROFILE") {
                Ok(name) => Some(name),
                Err(_) => None,
            }
        });

    let result = do_it(
        matches.values_of("files").unwrap(),
        config,
        profile_path,
        profile_name
    );
    if let Err(error) = result {
        eprintln!("{}", error);
        process::exit(1);
    };
}

fn do_it(
    file_values: Values,
    config: Config,
    profile_path: Option<String>,
    profile_name: Option<String>
) -> Result<()> {
    let profile = parse_profile(profile_path, profile_name)?;
    let requests: Vec<Request> = validate_and_parse_files(file_values)?;
    let mut preprocessor = Requestpreprocessor::new(profile, requests, config)?;
    let client = Client::new();

    while !preprocessor.is_empty() {
        let req: Result<Request> = preprocessor.next().unwrap();
        let req = req?;
        let dependency = req.dependency;

        let path = req.source_path.clone();

        config.log(1, format!("calling '{}'... ", path.to_str().unwrap()));
        let resp = client.exec(req)?;
        config.logln(1, format!("{}", resp.status()));

        if !resp.status().is_success() {
            if resp.body().trim().is_empty() {
                eprintln!("no response body");
            } else {
                eprintln!("{}", resp.body());
            }
            std::process::exit(1);
        }

        preprocessor.notify_response(&path, resp.body());

        if !dependency {
            println!("{}", resp.body());
        }
    }

    Ok(())
}

fn validate_and_parse_files(values: Values) -> Result<Vec<Request>> {
    let files = values
        .map(|file| PathBuf::from_str(file).unwrap())
        .collect::<Vec<_>>();

    let non_existent = files.iter()
        .filter(|it| !it.exists())
        .collect::<Vec<_>>();

    if !non_existent.is_empty() {
        for file in non_existent {
            eprintln!("'{}' does not exist", file.to_str().unwrap())
        }
        process::exit(1);
    }

    let non_file = files.iter()
        .filter(|it| !it.is_file())
        .collect::<Vec<_>>();

    if !non_file.is_empty() {
        for file in non_file {
            eprintln!("'{}' is not a file", file.to_str().unwrap())
        }
        process::exit(1);
    }

    let mut ret = vec![];
    for file in files {
        ret.push(Request::from_file(&file, false)?);
    }

    Ok(ret)
}

fn parse_profile(
    profile_path: Option<String>,
    profile: Option<String>
) -> Result<Profile> {
    let profile_path = profile_path.map(|it| PathBuf::from_str(&it).unwrap());

    let path = match profile_path {
        Some(profile_path) => {
            match profile_path.exists() {
                true => Ok(profile_path),
                false => Err(FhttpError::new(format!("file not found: '{}'", profile_path.to_str().unwrap())))
            }
        },
        None => {
            let profile_path = PathBuf::from_str("fhttp-config.json").unwrap();
            match profile_path.exists() {
                true => Ok(profile_path),
                false => return Ok(Profile::empty(env::current_dir().unwrap()))
            }
        }
    }?;

    let mut profiles = Profiles::parse(&path)?;
    let mut default = profiles.remove("default")
        .unwrap_or(Profile::empty(&path));
    let profile = match profile {
        Some(ref name) => profiles.remove(name).ok_or(FhttpError::new("profile not found"))?,
        None => Profile::empty(&path),
    };

    default.override_with(profile);
    Ok(default)
}
