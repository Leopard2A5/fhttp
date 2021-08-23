use crate::errors::Result;
use std::ops::Range;

pub trait Evaluation {
    fn num_backslashes(&self) -> usize;

    fn range(&self) -> &Range<usize>;

    fn is_escaped(&self) -> bool {
        self.num_backslashes() % 2 != 0
    }

    fn replace<S: Into<String>, F: FnOnce() -> Result<S>>(
        &self,
        target: &mut String,
        producer: F,
    ) -> Result<()> {
        let backslashes = self.num_backslashes();
        let range = self.range();

        if self.is_escaped() {
            target.replace_range(
                range.start..=range.start + backslashes / 2,
                ""
            );
        } else {
            let start = range.start + backslashes / 2;
            let text = producer()?.into();
            let end_index = match text.chars().last() {
                Some('\n') => text.len() - 1,
                _ => text.len(),
            };
            target.replace_range(start..range.end, &text[0..end_index]);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct BaseEvaluation {
    pub backslashes: usize,
    pub range: Range<usize>,
}

impl BaseEvaluation {
    pub fn new(
        range: Range<usize>,
        backslashes: usize,
    ) -> Self {
        BaseEvaluation {
            range,
            backslashes,
        }
    }
}

impl AsRef<BaseEvaluation> for BaseEvaluation {
    fn as_ref(&self) -> &BaseEvaluation {
        self
    }
}

impl <T: AsRef<BaseEvaluation>> Evaluation for T {
    fn num_backslashes(&self) -> usize {
        self.as_ref().backslashes
    }

    fn range(&self) -> &Range<usize> {
        &self.as_ref().range
    }
}
