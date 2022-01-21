use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::{App, Arg, crate_authors, crate_version, value_t, Values};
use itertools::Itertools;

use fhttp_core::{Config, Profile, Profiles, RequestSource};
use fhttp_core::Client;
use fhttp_core::execution::curl::Curl;
use fhttp_core::path_utils::CanonicalizedPathBuf;
use fhttp_core::Requestpreprocessor;

fn main() -> Result<()> {
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
            .long("--verbose")
            .multiple(true)
            .help("sets the level of verbosity"))
        .arg(Arg::with_name("quiet")
            .short("q")
            .long("quiet")
            .help("suppress log outputs")
            .conflicts_with("v"))
        .arg(Arg::with_name("print-paths")
            .short("P")
            .long("print-paths")
            .help("print request file paths instead of method and url"))
        .arg(Arg::with_name("timeout-ms")
            .short("t")
            .long("timeout-ms")
            .takes_value(true)
            .help("time out after this many ms on each request"))
        .arg(Arg::with_name("curl")
            .short("c")
            .long("curl")
            .help("print curl commands instead of executing given requests. Dependencies are still executed."))
        .get_matches();

    let config = Config::new(
        !matches.is_present("no-prompt"),
        matches.occurrences_of("v") as u8 + 1,
        matches.is_present("quiet"),
        matches.is_present("print-paths"),
        value_t!(matches, "timeout-ms", u64)
            .ok()
            .map(Duration::from_millis),
        matches.is_present("curl"),
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

    do_it(
        matches.values_of("files").unwrap(),
        config,
        profile_path,
        profile_name
    )
}

fn do_it(
    file_values: Values,
    config: Config,
    profile_path: Option<String>,
    profile_name: Option<String>
) -> Result<()> {
    let profile = parse_profile(profile_path, profile_name)?;
    let requested_files = file_values
        .map(|file| PathBuf::from_str(file).unwrap())
        .collect::<Vec<_>>();
    let requests: Vec<RequestSource> = validate_and_parse_files(&requested_files)?;

    check_curl_requested_for_dependencies(
        &config,
        &requested_files,
        &requests,
    )?;

    let mut preprocessor = Requestpreprocessor::new(profile, requests, config)?;
    let client = Client::new();

    while !preprocessor.is_empty() {
        let req = preprocessor.next().unwrap()?;
        let dependency = req.dependency;
        let req = req.parse()?;
        let path = req.source_path;
        let req = req.request;

        let msg = match config.print_file_paths() {
            true => format!("{}... ", &path.to_str()),
            false => format!("{} {}... ", &req.method, req.url),
        };

        config.log(1, msg);
        if config.curl() && !dependency {
            println!("\n{}", req.curl());
        } else {
            let resp = client.exec(
                req.method,
                &req.url,
                req.headers,
                req.body,
                req.response_handler,
                config.timeout(),
            )?;
            config.logln(1, format!("{}", resp.status()));

            if !resp.status().is_success() {
                let msg = if resp.body().trim().is_empty() {
                    "no response body"
                } else {
                    resp.body()
                };
                return Err(anyhow!("{}", msg));
            }

            preprocessor.notify_response(&path, resp.body());

            if !dependency {
                println!("{}", resp.body());
            }
        }
    }

    Ok(())
}

fn validate_and_parse_files(files: &[PathBuf]) -> Result<Vec<RequestSource>> {
    let non_existent = files.iter()
        .filter(|it| !it.exists())
        .collect::<Vec<_>>();

    if !non_existent.is_empty() {
        let msg = non_existent.iter()
            .map(|file| format!("'{}' does not exist", file.to_str().unwrap()))
            .join("\n");
        return Err(anyhow!("{}", msg));
    }

    let non_file = files.iter()
        .filter(|it| !it.is_file())
        .collect::<Vec<_>>();

    if !non_file.is_empty() {
        let msg = non_file.iter()
            .map(|file| format!("'{}' is not a file", file.to_str().unwrap()))
            .join("\n");
        return Err(anyhow!("{}", msg));
    }

    let mut ret = vec![];
    for file in files {
        ret.push(RequestSource::from_file(&file, false)?);
    }

    Ok(ret)
}

fn check_curl_requested_for_dependencies(
    config: &Config,
    requested_files: &[PathBuf],
    requests: &[RequestSource],
) -> Result<()> {
    use fhttp_core::path_utils;

    if config.curl() {
        let requested_files = requested_files.iter()
            .map(|it| path_utils::canonicalize(it))
            .collect::<Result<Vec<CanonicalizedPathBuf>>>()?;
        let dependencies = requests.iter()
            .map(|req| Ok((req.source_path.clone(), req.dependencies()?)))
            .collect::<Result<Vec<(CanonicalizedPathBuf, Vec<CanonicalizedPathBuf>)>>>()?;
        let dependencies = dependencies.into_iter()
            .flat_map(|(source, deps)| {
                deps.into_iter()
                    .map(|dep| (dep, source.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<HashMap<_, _>>();

        for possible_dependency in requested_files {
            if let Some(dependency_of) = dependencies.get(&possible_dependency) {
                return Err(
                    anyhow!(
                        "{}\nis a dependency of\n{}.\nIf you want me to print the curl snippet for both requests you'll need to do them separately.",
                        possible_dependency.to_str(),
                        dependency_of.to_str(),
                    )
                )
            }
        }
    }

    Ok(())
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
                false => Err(anyhow!("file not found: '{}'", profile_path.to_str().unwrap()))
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
        .unwrap_or_else(|| Profile::empty(&path));
    let profile = match profile {
        Some(ref name) => profiles.remove(name).ok_or_else(|| anyhow!("profile '{}' not found in '{}'", name, path.to_str().unwrap()))?,
        None => Profile::empty(&path),
    };

    default.override_with(profile);
    Ok(default)
}
