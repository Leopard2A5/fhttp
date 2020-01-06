use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use clap::{App, Arg, crate_authors, crate_version, Values};

use fhttp::{Client, Request, RequestPreprocessor, Result};

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

    let result= do_it(matches.values_of("files").unwrap());
    if let Err(error) = result {
        println!("{}", error);
        process::exit(1);
    };
}

fn do_it(file_values: Values) -> Result<()> {
    let requests: Vec<Request> = validate_and_parse_files(file_values)?;
    let mut preprocessor = RequestPreprocessor::new(requests)?;
    let client = Client::new();

    while !preprocessor.is_empty() {
        let req = preprocessor.next().unwrap();

        let path = req.source_path.clone();
        let resp = client.exec(req);
        preprocessor.notify_response(&path, resp.body());
        println!("{:?}\n##################\n{}", &path, resp.body());
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
        ret.push(Request::parse(content, &file));
    }

    Ok(ret)
}
