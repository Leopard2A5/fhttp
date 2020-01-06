extern crate reqwest;
#[macro_use]
extern crate lazy_static;
extern crate indoc;
extern crate jsonpath_lib as jsonpath;
extern crate serde_json;
extern crate clap;

mod request;
mod response_handler;
mod client;
mod request_preprocessor;
mod response;
mod errors;

pub type Result<T> = std::result::Result<T, FhttpError>;

pub use request::Request;
pub use client::Client;
pub use request_preprocessor::RequestPreprocessor;
pub use response::Response;
pub use errors::FhttpError;
