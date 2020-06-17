extern crate reqwest;
#[allow(unused)]
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

mod client;

pub use client::Client;
