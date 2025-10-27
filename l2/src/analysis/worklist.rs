use l2::*;
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Default)]
pub struct Worklist<'a> {
    queue: VecDeque<&'a BlockId>,
    set: HashSet<&'a BlockId>,
}

impl<'a> Worklist<'a> {
    pub fn extend<I: IntoIterator<Item = &'a BlockId>>(&mut self, indexes: I) {
        for i in indexes {
            if self.set.insert(i) {
                self.queue.push_back(i);
            }
        }
    }

    pub fn pop(&mut self) -> Option<&'a BlockId> {
        if let Some(index) = self.queue.pop_front() {
            self.set.remove(&index);
            Some(index)
        } else {
            None
        }
    }
}
