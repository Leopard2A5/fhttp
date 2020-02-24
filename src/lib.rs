extern crate reqwest;
#[macro_use]
extern crate lazy_static;
extern crate jsonpath_lib as jsonpath;
extern crate serde;
extern crate serde_json;
extern crate clap;
extern crate promptly;
extern crate rand;
extern crate uuid;
#[cfg(test)]
extern crate indoc;
#[cfg(test)]
extern crate maplit;

mod request;
mod response_handler;
mod client;
mod request_preprocessor;
mod response;
mod errors;
mod config;
mod profiles;
mod random_numbers;
mod uuids;

pub type Result<T> = std::result::Result<T, FhttpError>;

pub use request::Request;
pub use client::Client;
pub use request_preprocessor::Requestpreprocessor;
pub use response::Response;
pub use errors::{FhttpError, ErrorKind};
pub use config::Config;
pub use profiles::{Profiles, Profile};
