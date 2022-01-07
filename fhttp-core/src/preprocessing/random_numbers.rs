#[cfg(test)]
use std::cell::RefCell;
use std::ops::Range;

use anyhow::{anyhow, Result};

use crate::preprocessing::evaluation::BaseEvaluation;

#[cfg(test)]
thread_local!(
    pub static RANDOM_INT_CALLS: RefCell<Vec<(i32, i32)>> = RefCell::new(vec![])
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

pub fn parse_min_max(
    min: Option<&str>,
    max: Option<&str>
) -> Result<(i32, i32)> {
    let ret_min = min
        .map(|m| m.parse::<i32>())
        .unwrap_or(Ok(0))
        .map_err(|_| anyhow!("min param out of bounds: {}..{}", i32::MIN, i32::MAX))?;
    let ret_max = max
        .map(|m| m.parse::<i32>())
        .unwrap_or(Ok(std::i32::MAX))
        .map_err(|_| anyhow!("max param out of bounds: {}..{}", i32::MIN, i32::MAX))?;

    if ret_max < ret_min {
        Err(anyhow!("min cannot be greater than max"))
    } else {
        Ok((ret_min, ret_max))
    }
}

pub struct RandomNumberEval<'a> {
    pub min: Option<&'a str>,
    pub max: Option<&'a str>,
    pub base_eval: BaseEvaluation,
}

impl<'a> RandomNumberEval<'a> {
    pub fn new(
        min: Option<&'a str>,
        max: Option<&'a str>,
        range: Range<usize>,
        backslashes: usize,
    ) -> Self {
        RandomNumberEval {
            min,
            max,
            base_eval: BaseEvaluation::new(range, backslashes),
        }
    }
}

impl<'a> AsRef<BaseEvaluation> for RandomNumberEval<'a> {
    fn as_ref(&self) -> &BaseEvaluation {
        &self.base_eval
    }
}
