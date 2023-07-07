#![allow(unused_variables)]

use alpm::{AnyQuestion, Question::*};

#[derive(Clone)]
pub struct QueryCallback;

impl QueryCallback {
    pub fn new() -> Self {
        Self
    }
}

pub fn questioncb(question: AnyQuestion, this: &mut QueryCallback) {
    match question.question() {
        Conflict(x) => (),
        Replace(x) => (),
        Corrupted(x) => (),
        RemovePkgs(x) => (),
        ImportKey(x) => (),
        _ => ()
    }
}
