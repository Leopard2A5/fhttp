extern crate reqwest;
#[macro_use]
extern crate lazy_static;
extern crate indoc;
extern crate jsonpath_lib as jsonpath;
extern crate serde;
extern crate serde_json;
extern crate clap;
extern crate promptly;
extern crate maplit;
extern crate rand;
extern crate uuid;

mod request;
mod request2;
mod response_handler;
mod client;
mod request_preprocessor;
mod request_preprocessor2;
mod response;
mod errors;
mod config;
mod profiles;
mod random_numbers;
mod uuids;

pub type Result<T> = std::result::Result<T, FhttpError>;

pub use request::Request;
pub use client::Client;
pub use request_preprocessor::RequestPreprocessor;
pub use response::Response;
pub use errors::{FhttpError, ErrorKind};
pub use config::Config;
pub use profiles::{Profiles, Profile};
