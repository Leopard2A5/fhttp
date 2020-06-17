#[allow(unused)]
use std::cell::RefCell;
use regex::{Regex, Captures, Match};
use crate::{Result, FhttpError};

#[cfg(test)]
thread_local!(
    static RANDOM_INT_CALLS: RefCell<Vec<(i32, i32)>> = RefCell::new(vec![])
);

#[cfg(not(test))]
#[allow(unused)]
pub fn random_int(
    min: i32,
    max: i32
) -> i32 {
    use rand::{thread_rng, Rng};

    thread_rng().gen_range::<i32, i32, i32>(min, max)
}

#[cfg(test)]
#[allow(unused)]
pub fn random_int(
    min: i32,
    max: i32
) -> i32 {
    RANDOM_INT_CALLS.with(|c| {
        c.borrow_mut().push((min, max));
    });
    7i32
}

pub fn replace_random_ints(text: String) -> Result<String> {
    lazy_static! {
        static ref RE_ENV: Regex = Regex::new(r"(?m)\$\{randomInt\(\s*([+-]?\d+)?\s*(,\s*([+-]?\d+)\s*)?\)}").unwrap();
    };

    let reversed_captures: Vec<Captures> = RE_ENV.captures_iter(&text)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if reversed_captures.is_empty() {
        Ok(text)
    } else {
        let mut buffer = text.to_owned();

        for capture in reversed_captures {
            let group = capture.get(0).unwrap();
            let (min, max) = parse_min_max(
                capture.get(1),
                capture.get(3)
            )?;

            let range = group.start()..group.end();
            let value = random_int(min, max);

            buffer.replace_range(range, &value.to_string());
        }

        Ok(buffer)
    }
}

fn parse_min_max(
    min: Option<Match>,
    max: Option<Match>
) -> Result<(i32, i32)> {
    let ret_min = min
        .map(|m| m.as_str().parse::<i32>())
        .unwrap_or(Ok(0))
        .map_err(|_| FhttpError::new(
            format!("min param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)
        ))?;
    let ret_max = max
        .map(|m| m.as_str().parse::<i32>())
        .unwrap_or(Ok(std::i32::MAX))
        .map_err(|_| FhttpError::new(
            format!("max param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX)
        ))?;

    if ret_max < ret_min {
        Err(FhttpError::new("min cannot be greater than max"))
    } else {
        Ok((ret_min, ret_max))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{FhttpError, Result};

    #[test]
    fn test_happy_path() -> Result<()> {
        let buffer = String::from("${randomInt()}");
        let result = replace_random_ints(buffer)?;
        assert_eq!(result, "7");

        let buffer = String::from("${randomInt(-5)}");
        let result = replace_random_ints(buffer)?;
        assert_eq!(result, "7");

        let buffer = String::from("${randomInt(-5, 7)}");
        let result = replace_random_ints(buffer)?;
        assert_eq!(result, "7");

        RANDOM_INT_CALLS.with(|calls| {
            assert_eq!(*calls.borrow(), vec![
                (0, std::i32::MAX),
                (-5, std::i32::MAX),
                (-5, 7)
            ]);
        });

        Ok(())
    }

    #[test]
    fn test_invalid_min() {
        let buffer = format!("${{randomInt({})}}", std::i32::MIN as i64 - 1);
        let result = replace_random_ints(buffer);
        assert_eq!(result, Err(FhttpError::new(format!("min param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX))));
    }

    #[test]
    fn test_invalid_max() {
        let buffer = format!("${{randomInt(0, {})}}", std::i32::MAX as i64 + 1);
        let result = replace_random_ints(buffer);
        assert_eq!(result, Err(FhttpError::new(format!("max param out of bounds: {}..{}", std::i32::MIN, std::i32::MAX))));
    }

    #[test]
    fn test_min_gt_max() {
        let buffer = "${randomInt(3, 2)}".to_owned();
        let result = replace_random_ints(buffer);
        assert_eq!(result, Err(FhttpError::new("min cannot be greater than max")));
    }
}
