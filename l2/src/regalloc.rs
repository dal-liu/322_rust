mod coloring;
mod interference;
mod spilling;

use std::collections::HashSet;

use l2::*;

use crate::analysis::{compute_dominators, compute_liveness, compute_loops};

use coloring::{ColoringResult, color_graph};
use interference::build_interference;
use spilling::spill;

pub fn allocate_registers(func: &mut Function, interner: &mut Interner<String>) {
    let prefix = "S";
    let mut suffix = 0;
    let mut prev_spilled = HashSet::new();

    loop {
        let liveness = compute_liveness(func);
        let mut interference = build_interference(func, &liveness);
        let dominators = compute_dominators(func);
        let loops = compute_loops(func, &dominators);
        let coloring = color_graph(func, &liveness, &mut interference, &loops, &prev_spilled);

        if coloring.spill_nodes.is_empty() {
            rewrite_program(func, &coloring);
            break;
        }

        for var in coloring.spill_nodes {
            let var = coloring.interner.resolve(var);
            let spilled = spill(func, var, prefix, &mut suffix, interner);
            prev_spilled.extend(spilled.into_iter());
        }
    }
}

fn rewrite_program(func: &mut Function, coloring: &ColoringResult) {
    func.basic_blocks
        .iter_mut()
        .flat_map(|block| &mut block.instructions)
        .for_each(|inst| {
            inst.defs()
                .into_iter()
                .chain(inst.uses())
                .filter(|val| matches!(val, Value::Variable(_)))
                .for_each(|var| {
                    let color = coloring.color[&coloring.interner[&var]];
                    inst.replace_value(&var, coloring.interner.resolve(color));
                })
        });
}
