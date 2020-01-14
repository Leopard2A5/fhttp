use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use clap::{App, Arg, crate_authors, crate_version, Values};

use fhttp::{Client, Request, RequestPreprocessor, Result, Config};

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
        .get_matches();

    let config = Config {
        prompt_missing_env_vars: !matches.is_present("no-prompt"),
    };

    let result= do_it(
        matches.values_of("files").unwrap(),
        config
    );
    if let Err(error) = result {
        println!("{}", error);
        process::exit(1);
    };
}

fn do_it(
    file_values: Values,
    config: Config
) -> Result<()> {
    let requests: Vec<Request> = validate_and_parse_files(file_values)?;
    let mut preprocessor = RequestPreprocessor::new(requests, config)?;
    let client = Client::new();

    while !preprocessor.is_empty() {
        let req = preprocessor.next().unwrap();
        let dependency = req.dependency;

        let path = req.source_path.clone();
        eprint!("calling '{}'... ", path.to_str().unwrap());
        let resp = client.exec(req);
        eprintln!("{}", resp.status());
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
        let content = std::fs::read_to_string(&file)?;
        ret.push(Request::parse(content, &file)?);
    }

    Ok(ret)
}
