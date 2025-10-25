use l2::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub struct AnalysisResult {
    pub gen_set: Vec<Vec<HashSet<Value>>>,
    pub kill_set: Vec<Vec<HashSet<Value>>>,
    pub in_set: Vec<Vec<HashSet<Value>>>,
    pub out_set: Vec<Vec<HashSet<Value>>>,
}

#[derive(Debug)]
struct Worklist {
    queue: VecDeque<usize>,
    set: HashSet<usize>,
}

impl Worklist {
    pub fn push(&mut self, index: usize) {
        if self.set.insert(index) {
            self.queue.push_back(index);
        }
    }

    pub fn pop(&mut self) -> Option<usize> {
        if let Some(index) = self.queue.pop_front() {
            self.set.remove(&index);
            Some(index)
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/* fn compute_liveness(func: &Function) -> AnalysisResult {
    // TODO: this
} */

pub fn compute_targets(func: &mut Function) {
    let blocks = &mut func.basic_blocks;
    let last_index = blocks.len().saturating_sub(1);

    let label_to_index: HashMap<_, _> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, block)| {
            if let Some(Instruction::Label(label)) = block.instructions.first() {
                Some((label.clone(), i))
            } else {
                None
            }
        })
        .collect();

    for i in 0..blocks.len() {
        let block = &mut blocks[i];
        let last_inst = block.instructions.last();

        block.target = match last_inst {
            Some(Instruction::CJump { label, .. }) => {
                let j = label_to_index
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("invalid label {}", label));
                let k = if i < last_index { Some(i + 1) } else { None };
                Target::Indexes([Some(j), k])
            }
            Some(Instruction::Goto(label)) => {
                let j = label_to_index
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("invalid label {}", label));
                Target::Indexes([Some(j), None])
            }
            Some(Instruction::Return) => Target::Indexes([None, None]),
            Some(_) => {
                let k = if i < last_index { Some(i + 1) } else { None };
                Target::Indexes([k, None])
            }
            None => panic!("empty block in {}", &func.name),
        }
    }
}
