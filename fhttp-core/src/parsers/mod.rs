mod parsed_request;
mod parsing;
mod parsing_gql;

pub use parsed_request::ParsedRequest;

pub use parsing::parse_str;
pub use parsing_gql::parse_gql_str;

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
