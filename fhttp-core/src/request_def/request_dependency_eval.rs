use crate::evaluation::BaseEvaluation;
use std::ops::Range;

pub struct RequestDependencyEval<'a> {
    pub path: &'a str,
    pub base_eval: BaseEvaluation,
}

impl <'a> RequestDependencyEval<'a> {
    pub fn new(
        path: &'a str,
        range: Range<usize>,
        backslashes: usize,
    ) -> Self {
        RequestDependencyEval {
            path,
            base_eval: BaseEvaluation::new(range, backslashes),
        }
    }
}

impl <'a> AsRef<BaseEvaluation> for RequestDependencyEval<'a> {
    fn as_ref(&self) -> &BaseEvaluation {
        &self.base_eval
    }
}
