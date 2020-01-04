use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use clap::{App, Arg, crate_authors, crate_version, Values};

use fhttp::{Client, Request};

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
        .get_matches();

    let requests = validate_and_parse_files(matches.values_of("files").unwrap());

    let client = Client::new();
    for req in requests {
        println!("{:#?}", req);
        client.exec(req);
    }
}

fn validate_and_parse_files(values: Values) -> Vec<Request> {
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

    files.into_iter()
        .map(|file| {
            let content = std::fs::read_to_string(&file).unwrap();
            Request::parse(content, &file)
        })
        .collect()
}
