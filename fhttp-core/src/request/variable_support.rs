use std::ops::Range;
use crate::Request;
use regex::{Captures, Regex};

pub trait VariableSupport {
    fn get_env_vars(&self) -> Vec<(&str, Range<usize>)>;
}

impl VariableSupport for Request {
    fn get_env_vars(&self) -> Vec<(&str, Range<usize>)> {
        lazy_static! {
            static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{env\(([^}]+)\)}").unwrap();
        };

        RE_ENV.captures_iter(&self.text)
            .collect::<Vec<Captures>>()
            .into_iter()
            .rev()
            .map(|capture| {
                let group = capture.get(0).unwrap();
                let key = capture.get(1).unwrap().as_str();
                (key, group.start()..group.end())
            })
            .collect()
    }
}
