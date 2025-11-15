use l2::*;
use std::collections::{HashSet, VecDeque};

#[derive(Debug)]
pub struct Worklist<'a> {
    queue: VecDeque<&'a BlockId>,
    set: HashSet<&'a BlockId>,
}

impl<'a> Worklist<'a> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    pub fn pop(&mut self) -> Option<&'a BlockId> {
        self.queue.pop_front().and_then(|id| {
            self.set.remove(id);
            Some(id)
        })
    }

    pub fn push(&mut self, id: &'a BlockId) {
        if self.set.insert(id) {
            self.queue.push_back(id);
        }
    }
}

impl<'a> Extend<&'a BlockId> for Worklist<'a> {
    fn extend<T: IntoIterator<Item = &'a BlockId>>(&mut self, iter: T) {
        for id in iter {
            self.push(id);
        }
    }
}
