use std::borrow::Cow;
use regex::{Regex, Captures};
use uuid::Uuid;

pub fn replace_uuids<'a, T: Into<Cow<'a, str>>>(text: T) -> Cow<'a, str> {
    let cow = text.into();

    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{uuid\(\)}").unwrap();
    };

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&cow)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if reversed_captures.is_empty() {
        cow
    } else {
        let mut buffer = str::to_owned(&cow);

        for capture in reversed_captures {
            let group = capture.get(0).unwrap();
            let range = group.start()..group.end();
            let value = Uuid::new_v4().to_string();

            buffer.replace_range(range, &value);
        }

        Cow::Owned(buffer)
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

        let input = "foo ${uuid()}";
        let result = replace_uuids(input);
        assert!(REGEX.is_match(&result));
    }
}
