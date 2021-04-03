use std::fmt::{Display, Formatter};
use std::collections::LinkedList;
use crate::error::CompilationError;

pub struct Errors {
    list: LinkedList<CompilationError>
}

impl Errors {
    pub fn new() -> Self {
        Errors {
            list: LinkedList::new()
        }
    }

    pub fn push(&mut self, err: CompilationError) {
        self.list.push_back(err)
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }
}

impl Display for Errors {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if let Some(e) = self.list.iter().nth(0) {
            write!(f, "{}", e)?
        }

        for e in self.list.iter().skip(1) {
            write!(f, "\n{}", e)?
        }

        Ok(())
    }
}
