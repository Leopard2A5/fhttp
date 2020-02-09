use std::cell::RefCell;
use rand::{thread_rng, Rng};
use regex::{Regex, Captures, Match};
use crate::{Result, FhttpError, ErrorKind};

#[allow(unused)]
type RandomIntFunction = Box<dyn Fn(u32, u32) -> u32>;

thread_local!(
    pub static RANDOM_INT_GENERATOR: RefCell<RandomIntFunction> = RefCell::new(Box::new(random_int))
);

#[allow(unused)]
fn random_int(
    min: u32,
    max: u32
) -> u32 {
    thread_rng().gen_range::<u32, u32, u32>(min, max)
}

pub fn replace_random_ints(text: &str) -> Result<String> {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{randomInt\(\s*([+-]?\d+)?\s*(,\s*([+-]?\d+)\s*)?\)}").unwrap();
    };
    let mut buffer = text.to_owned();

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    for capture in reversed_captures {
        let group = capture.get(0).unwrap();
        let (min, max) = parse_min_max(
            capture.get(1),
            capture.get(3)
        )?;

        let range = group.start()..group.end();
        let value = RANDOM_INT_GENERATOR.with(|gen| {
            gen.borrow()(min, max)
        });

        buffer.replace_range(range, &value.to_string());
    }

    Ok(buffer)
}

fn parse_min_max(
    min: Option<Match>,
    max: Option<Match>
) -> Result<(u32, u32)> {
    let ret_min = min
        .map(|m| m.as_str().parse::<u32>())
        .unwrap_or(Ok(0))
        .map_err(|_| FhttpError::new(ErrorKind::RequestParseException(
            format!("min param out of bounds: 0..{}", std::u32::MAX)
        )))?;
    let ret_max = max
        .map(|m| m.as_str().parse::<u32>())
        .unwrap_or(Ok(std::u32::MAX))
        .map_err(|_| FhttpError::new(ErrorKind::RequestParseException(
            format!("max param out of bounds: 0..{}", std::u32::MAX)
        )))?;

    if ret_max < ret_min {
        Err(FhttpError::new(ErrorKind::RequestParseException(String::from(
            "min cannot be greater than max"
        ))))
    } else {
        Ok((ret_min, ret_max))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{FhttpError, ErrorKind};

    #[test]
    fn test_happy_path() {
        thread_local!(
            static CALLS: RefCell<Vec<(u32, u32)>> = RefCell::new(vec![])
        );

        let lambda = |min: u32, max: u32| {
            CALLS.with(|c| {
                c.borrow_mut().push((min, max));
            });
            7u32
        };
        let replacement: RandomIntFunction = Box::new(lambda);

        RANDOM_INT_GENERATOR.with(|f| {
            f.replace(replacement);
        });

        let buffer = String::from("${randomInt()}");
        let result = replace_random_ints(&buffer);
        assert_eq!(result.unwrap(), "7");

        let buffer = String::from("${randomInt(5)}");
        let result = replace_random_ints(&buffer);
        assert_eq!(result.unwrap(), "7");

        let buffer = String::from("${randomInt(5, 12)}");
        let result = replace_random_ints(&buffer);
        assert_eq!(result.unwrap(), "7");

        CALLS.with(|calls| {
            assert_eq!(*calls.borrow(), vec![
                (0, std::u32::MAX),
                (5, std::u32::MAX),
                (5, 12)
            ]);
        });
    }

    #[test]
    fn test_invalid_min() {
        let buffer = "${randomInt(-1)}";
        let result = replace_random_ints(&buffer);
        match result {
            Err(FhttpError { kind: ErrorKind::RequestParseException(e) }) => {
                assert_eq!(e, format!("min param out of bounds: 0..{}", std::u32::MAX))
            },
            _ => panic!("expected error!")
        }
    }

    #[test]
    fn test_invalid_max() {
        let buffer = "${randomInt(0, -3)}";
        let result = replace_random_ints(&buffer);
        match result {
            Err(FhttpError { kind: ErrorKind::RequestParseException(e) }) => {
                assert_eq!(e, format!("max param out of bounds: 0..{}", std::u32::MAX))
            },
            _ => panic!("expected error!")
        }
    }

    #[test]
    fn test_min_gt_max() {
        let buffer = "${randomInt(3, 2)}";
        let result = replace_random_ints(&buffer);
        match result {
            Err(FhttpError { kind: ErrorKind::RequestParseException(e) }) => {
                assert_eq!(e, String::from("min cannot be greater than max"))
            },
            _ => panic!("expected error!")
        }
    }
}
