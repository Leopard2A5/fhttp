extern crate reqwest;
extern crate jsonpath_lib as jsonpath;
extern crate serde_json;
extern crate regex;
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
extern crate indoc;

pub mod path_utils;
mod config;
mod request;
mod errors;
mod response_handler;

pub use config::Config;
pub use request::{Request, RE_REQUEST};
pub use errors::{FhttpError, Result};
pub use response_handler::{ResponseHandler, JsonPathResponseHandler};
