use regex::{Regex, Captures};
use uuid::Uuid;

pub fn replace_uuids(text: String) -> String {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{uuid\(\)}").unwrap();
    };

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if reversed_captures.is_empty() {
        text
    } else {
        let mut buffer = text.clone();

        for capture in reversed_captures {
            let group = capture.get(0).unwrap();
            let range = group.start()..group.end();
            let value = Uuid::new_v4().to_string();

            buffer.replace_range(range, &value);
        }

        buffer
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_replace_uuids() {
        lazy_static! {
            static ref REGEX: Regex = Regex::new(r"foo [a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}").unwrap();
        };

        let input = "foo ${uuid()}".to_owned();
        let result = replace_uuids(input);
        assert!(REGEX.is_match(&result));
    }
}
