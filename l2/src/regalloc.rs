mod coloring;
mod interference;
mod spilling;

use crate::analysis::compute_liveness;
use crate::regalloc::coloring::{ColoringResult, color_graph};
use crate::regalloc::spilling::spill;

use l2::*;
use std::collections::HashSet;

pub use crate::regalloc::interference::build_interference;

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
                    let index = coloring.interner.get(&var).unwrap();
                    let color = coloring.color[&index];
                    inst.replace_value(&var, coloring.interner.resolve(color));
                })
        });
}

pub fn allocate_registers(func: &mut Function, interner: &mut Interner<String>) {
    let prefix = "S";
    let mut suffix = 0;
    let mut all_spilled = HashSet::new();

    loop {
        let liveness = compute_liveness(func);
        let mut interference = build_interference(func, &liveness);
        let coloring = color_graph(func, &mut interference, &mut all_spilled);

        if coloring.spill_nodes.is_empty() {
            rewrite_program(func, &coloring);
            break;
        }

        for var in coloring.spill_nodes {
            let var = coloring.interner.resolve(var);
            let spilled = spill(func, var, prefix, &mut suffix, interner);
            all_spilled.extend(spilled.into_iter());
        }
    }
}
