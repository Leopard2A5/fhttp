use std::ops::Range;

use anyhow::Result;

pub trait Evaluation {
    fn backslashes(&self) -> &Range<usize>;

    fn range(&self) -> &Range<usize>;

    fn is_escaped(&self) -> bool {
        self.backslashes().len() % 2 != 0
    }

    fn replace<S: Into<String>, F: FnOnce() -> Result<S>>(
        &self,
        target: &mut String,
        producer: F,
    ) -> Result<()> {
        if !self.is_escaped() {
            let text = producer()?.into();
            let end_index = match text.chars().last() {
                Some('\n') => text.len() - 1,
                _ => text.len(),
            };
            target.replace_range(self.range().clone(), &text[0..end_index]);
        }
        escape_backslashes(target, self.backslashes());

        Ok(())
    }
}

fn escape_backslashes(target: &mut String, backslashes: &Range<usize>) {
    if backslashes.is_empty() {
        return;
    }

    let num_backslaches_to_remove = backslashes.len() / 2;
    let new_end = backslashes.end - num_backslaches_to_remove;

    target.replace_range(backslashes.start..new_end, "");
}

#[derive(Debug)]
pub struct BaseEvaluation {
    pub backslashes: Range<usize>,
    pub range: Range<usize>,
}

impl BaseEvaluation {
    pub fn new(range: Range<usize>, backslashes: Range<usize>) -> Self {
        BaseEvaluation { range, backslashes }
    }
}

impl AsRef<BaseEvaluation> for BaseEvaluation {
    fn as_ref(&self) -> &BaseEvaluation {
        self
    }
}

impl<T: AsRef<BaseEvaluation>> Evaluation for T {
    fn backslashes(&self) -> &Range<usize> {
        &self.as_ref().backslashes
    }

    fn range(&self) -> &Range<usize> {
        &self.as_ref().range
    }
}
