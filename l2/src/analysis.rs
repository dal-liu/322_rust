mod bitvector;
mod def_use;
mod value_interner;
mod worklist;

use crate::analysis::def_use::{defs, uses};
use crate::analysis::value_interner::ValueInterner;
use crate::analysis::worklist::Worklist;
use l2::*;
use std::collections::HashSet;

#[derive(Debug)]
pub struct AnalysisResult {
    pub gen_: Vec<Vec<HashSet<Value>>>,
    pub kill: Vec<Vec<HashSet<Value>>>,
    pub in_: Vec<Vec<HashSet<Value>>>,
    pub out: Vec<Vec<HashSet<Value>>>,
}

impl DisplayResolved for AnalysisResult {
    fn fmt_with(&self, f: &mut std::fmt::Formatter, interner: &StringInterner) -> std::fmt::Result {
        writeln!(f, "(\n(in")?;

        for vec in &self.in_ {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n(out")?;

        for vec in &self.out {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n)")
    }
}

pub fn compute_liveness(func: &Function) -> AnalysisResult {
    let value_map = ValueInterner::build(func);
    let num_values = value_map.len();
    let num_blocks = func.basic_blocks.len();
    let mut block_gen: Vec<HashSet<Value>> = vec![HashSet::new(); num_blocks];
    let mut block_kill: Vec<HashSet<Value>> = vec![HashSet::new(); num_blocks];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for inst in &block.instructions {
            block_gen[i].extend(
                uses(inst)
                    .into_iter()
                    .filter(|use_| !block_kill[i].contains(use_)),
            );
            block_kill[i].extend(defs(inst).into_iter());
        }
    }

    let cfg = &func.cfg;
    let mut block_in: Vec<HashSet<&Value>> = vec![HashSet::new(); num_blocks];
    let mut block_out: Vec<HashSet<&Value>> = vec![HashSet::new(); num_blocks];
    let mut worklist = Worklist::default();
    worklist.extend(func.basic_blocks.iter().map(|block| &block.id));

    while let Some(id) = worklist.pop() {
        block_out[id.0] = cfg.successors[id.0]
            .iter()
            .flat_map(|s| block_in[s.0].iter().copied())
            .collect();

        let temp: HashSet<&Value> = block_gen[id.0]
            .iter()
            .chain(
                block_out[id.0]
                    .iter()
                    .filter(|&val| !block_kill[id.0].contains(val))
                    .copied(),
            )
            .collect();

        if temp != block_in[id.0] {
            block_in[id.0] = temp;
            worklist.extend(cfg.predecessors[id.0].iter());
        }
    }

    let mut gen_ = empty_dataflow_set(func);
    let mut kill = empty_dataflow_set(func);
    let mut in_ = empty_dataflow_set(func);
    let mut out = empty_dataflow_set(func);

    for block in &func.basic_blocks {
        let i = block.id.0;

        for (j, inst) in block.instructions.iter().enumerate().rev() {
            gen_[i][j].extend(uses(inst).into_iter());
            kill[i][j].extend(defs(inst).into_iter());

            out[i][j] = if j == block.instructions.len() - 1 {
                block_out[i].iter().map(|&val| val.clone()).collect()
            } else {
                in_[i][j + 1].clone()
            };

            in_[i][j] = gen_[i][j]
                .union(&out[i][j].difference(&kill[i][j]).cloned().collect())
                .cloned()
                .collect();
        }
    }

    AnalysisResult {
        gen_,
        kill,
        in_,
        out,
    }
}

fn empty_dataflow_set(func: &Function) -> Vec<Vec<HashSet<Value>>> {
    func.basic_blocks
        .iter()
        .map(|block| vec![HashSet::new(); block.instructions.len()])
        .collect()
}
