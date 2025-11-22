use std::collections::HashMap;

use common::{BitVector, Interner};
use l3::*;

use crate::analysis::dataflow::{Dataflow, Direction, solve};

type InstId = usize;

#[derive(Debug)]
pub struct ReachingDefResult {
    pub interner: Interner<Instruction>,
    pub in_: Vec<Vec<BitVector>>,
}

pub fn compute_reaching_def(func: &mut Function) -> ReachingDefResult {
    let dummy = BasicBlock {
        id: BlockId(func.basic_blocks.len()),
        instructions: func
            .params
            .iter()
            .map(|&param| Instruction::Assign {
                dst: param,
                src: Value::Variable(param),
            })
            .collect(),
    };

    func.cfg.predecessors[0].push(dummy.id);
    func.cfg.successors.push(vec![BlockId(0)]);
    func.basic_blocks.push(dummy);

    let reaching_def = ReachingDefAnalysis::new(func);
    let block_in = solve(func, &reaching_def);

    let empty_dataflow_set = || -> Vec<Vec<BitVector>> {
        func.basic_blocks
            .iter()
            .map(|block| {
                vec![BitVector::new(reaching_def.interner.len()); block.instructions.len()]
            })
            .collect()
    };

    let mut inst_in = empty_dataflow_set();
    let mut inst_out = empty_dataflow_set();

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for (j, inst) in block.instructions.iter().enumerate().rev() {
            inst_in[i][j] = if j == 0 {
                block_in[i].clone()
            } else {
                inst_out[i][j - 1].clone()
            };

            inst_out[i][j] = inst_in[i][j].clone();
            if let Some(def) = inst.def() {
                inst_out[i][j].reset_from(reaching_def.def_table[&def].iter().copied());
                inst_out[i][j].set(j);
            }
        }
    }

    func.basic_blocks.pop();
    func.cfg.successors.pop();
    func.cfg.predecessors[0].pop();

    ReachingDefResult {
        interner: reaching_def.interner,
        in_: inst_in,
    }
}

#[derive(Debug)]
struct ReachingDefAnalysis {
    interner: Interner<Instruction>,
    def_table: HashMap<SymbolId, Vec<InstId>>,
    block_gen: Vec<BitVector>,
    block_kill: Vec<BitVector>,
}

impl ReachingDefAnalysis {
    pub fn new(func: &Function) -> Self {
        let (interner, def_table) = func
            .basic_blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|inst| inst.def().is_some())
            .fold(
                (Interner::new(), HashMap::new()),
                |(mut interner, mut def_table), inst| {
                    let index = interner.intern(inst.clone());
                    if let Some(def) = inst.def() {
                        def_table.entry(def).or_insert(Vec::new()).push(index);
                    }
                    (interner, def_table)
                },
            );

        let num_insts = interner.len();
        let num_blocks = func.basic_blocks.len();
        let mut block_gen = vec![BitVector::new(num_insts); num_blocks];
        let mut block_kill = vec![BitVector::new(num_insts); num_blocks];

        for (i, block) in func.basic_blocks.iter().enumerate() {
            block
                .instructions
                .iter()
                .rev()
                .filter(|inst| inst.def().is_some())
                .for_each(|inst| {
                    let j = interner[inst];
                    if !block_kill[i].test(j) {
                        block_gen[i].set(j);
                    }
                    block_kill[i]
                        .set_from(inst.def().iter().flat_map(|def| &def_table[def]).copied());
                });
        }

        ReachingDefAnalysis {
            interner,
            def_table,
            block_gen,
            block_kill,
        }
    }
}

impl Dataflow for ReachingDefAnalysis {
    const DIRECTION: Direction = Direction::Forward;

    fn boundary_condition(&self) -> BitVector {
        BitVector::new(self.interner.len())
    }

    fn meet(&self, current: &mut BitVector, other: &BitVector) {
        current.union(&other);
    }

    fn transfer(&self, input: &BitVector, id: BlockId) -> BitVector {
        let mut output = input.clone();
        output.difference(&self.block_kill[id.0]);
        output.union(&self.block_gen[id.0]);
        output
    }
}
