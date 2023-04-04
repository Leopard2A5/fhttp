mod parsing;
mod parsing_gql;

pub use crate::request::Request;

use lazy_regex::Regex;
pub use parsing::parse_str;
pub use parsing_gql::parse_gql_str;

pub fn fileupload_regex() -> &'static Regex {
    regex!(r##"(?m)\$\{\s*file\s*\(\s*"([^}]+)"\s*,\s*"([^}]+)"\s*\)\s*\}"##)
}

pub mod normal_parser {
    #[derive(Parser)]
    #[grammar = "parsers/grammar/request.pest"]
    pub struct RequestParser;
}

pub mod gql_parser {
    #[derive(Parser)]
    #[grammar = "parsers/grammar/gql_request.pest"]
    pub struct RequestParser;
}
