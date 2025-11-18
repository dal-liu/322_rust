use l2::*;
use std::collections::{HashSet, VecDeque};

#[derive(Debug)]
pub struct Worklist {
    queue: VecDeque<BlockId>,
    set: HashSet<BlockId>,
}

impl Worklist {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    pub fn pop(&mut self) -> Option<BlockId> {
        self.queue.pop_front().and_then(|id| {
            self.set.remove(&id);
            Some(id)
        })
    }

    pub fn push(&mut self, id: BlockId) {
        if self.set.insert(id) {
            self.queue.push_back(id);
        }
    }
}

impl Extend<BlockId> for Worklist {
    fn extend<T: IntoIterator<Item = BlockId>>(&mut self, iter: T) {
        for id in iter {
            self.push(id);
        }
    }
}
