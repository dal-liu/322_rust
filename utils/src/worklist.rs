use std::collections::{HashSet, VecDeque};
use std::hash::Hash;

#[derive(Debug)]
pub struct Worklist<T> {
    queue: VecDeque<T>,
    set: HashSet<T>,
}

impl<T: Copy + Eq + Hash> Worklist<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front().and_then(|id| {
            self.set.remove(&id);
            Some(id)
        })
    }

    pub fn push(&mut self, id: T) {
        if self.set.insert(id) {
            self.queue.push_back(id);
        }
    }
}

impl<T: Copy + Eq + Hash> Extend<T> for Worklist<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for id in iter {
            self.push(id);
        }
    }
}
