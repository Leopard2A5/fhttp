use regex::Regex;

lazy_static! {
    pub static ref RE_FILE: Regex = Regex::new(r##"(?m)\$\{\s*file\s*\(\s*"([^}]+)"\s*,\s*"([^}]+)"\s*\)\s*\}"##).unwrap();
}
