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

pub use request::Request;
pub use client::Client;
