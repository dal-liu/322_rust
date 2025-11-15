use l2::*;

use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

#[derive(Debug)]
pub struct DominatorsResult {
    pub dominators: Vec<BitVector>,
}

pub fn compute_dominators(func: &Function) -> DominatorsResult {
    let num_blocks = func.basic_blocks.len();
    let mut dominators = vec![BitVector::new(num_blocks); num_blocks];
    for i in 0..num_blocks {
        dominators[i].set_from(0..num_blocks);
    }

    let entry_block = func
        .basic_blocks
        .iter()
        .find(|block| func.cfg.predecessors[block.id.0].is_empty())
        .unwrap();

    let cfg = &func.cfg;
    let mut worklist = Worklist::new();
    worklist.push(&entry_block.id);

    while let Some(id) = worklist.pop() {
        let i = id.0;
        let mut temp = BitVector::new(num_blocks);

        if i != entry_block.id.0 {
            temp.set_from(0..num_blocks);
            for pred in &cfg.predecessors[i] {
                temp.intersection(&dominators[pred.0]);
            }
        }

        temp.set(i);

        if temp != dominators[i] {
            dominators[i] = temp;
            worklist.extend(cfg.successors[i].iter());
        }
    }

    DominatorsResult { dominators }
}
