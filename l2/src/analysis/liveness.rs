use l2::*;

use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

#[derive(Debug)]
pub struct LivenessResult {
    pub block_out: Vec<BitVector>,
    pub inst_out: Vec<Vec<BitVector>>,
    pub interner: Interner<Value>,
}

pub fn compute_liveness(func: &Function) -> LivenessResult {
    let mut interner = func
        .basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .flat_map(|inst| inst.uses().into_iter().chain(inst.defs()))
        .chain(Register::gp_registers().into_iter().map(Value::Register))
        .fold(Interner::new(), |mut interner, val| {
            interner.intern(val);
            interner
        });

    let num_gp_variables = interner.len();
    let num_blocks = func.basic_blocks.len();
    let mut block_gen: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut block_kill: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for inst in &block.instructions {
            block_gen[i].set_from(inst.uses().into_iter().filter_map(|use_| {
                let index = interner.intern(use_);
                (!block_kill[i].test(index)).then_some(index)
            }));
            block_kill[i].set_from(inst.defs().into_iter().map(|def| interner.intern(def)));
        }
    }

    let cfg = &func.cfg;
    let mut block_in: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut block_out: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut worklist = Worklist::new();
    worklist.extend((0..func.basic_blocks.len()).map(BlockId));

    while let Some(id) = worklist.pop() {
        let i = id.0;

        block_out[i].clear();
        for succ in &cfg.successors[i] {
            block_out[i].union(&block_in[succ.0]);
        }

        let mut temp = block_out[i].clone();
        temp.difference(&block_kill[i]);
        temp.union(&block_gen[i]);

        if temp != block_in[i] {
            block_in[i] = temp;
            worklist.extend(cfg.predecessors[i].iter().copied());
        }
    }

    let empty_dataflow_set = || -> Vec<Vec<BitVector>> {
        func.basic_blocks
            .iter()
            .map(|block| vec![BitVector::new(num_gp_variables); block.instructions.len()])
            .collect()
    };

    let mut inst_gen = empty_dataflow_set();
    let mut inst_kill = empty_dataflow_set();
    let mut inst_in = empty_dataflow_set();
    let mut inst_out = empty_dataflow_set();

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for (j, inst) in block.instructions.iter().enumerate().rev() {
            inst_gen[i][j].set_from(inst.uses().into_iter().map(|def| interner.intern(def)));
            inst_kill[i][j].set_from(inst.defs().into_iter().map(|def| interner.intern(def)));

            inst_out[i][j] = if j == block.instructions.len() - 1 {
                block_out[i].clone()
            } else {
                inst_in[i][j + 1].clone()
            };

            inst_in[i][j] = inst_out[i][j].clone();
            inst_in[i][j].difference(&inst_kill[i][j]);
            inst_in[i][j].union(&inst_gen[i][j]);
        }
    }

    LivenessResult {
        block_out,
        inst_out,
        interner,
    }
}
