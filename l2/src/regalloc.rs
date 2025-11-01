mod coloring;
mod interference;
mod spilling;

use crate::analysis::compute_liveness;
use crate::regalloc::coloring::ColoringResult;
use crate::regalloc::spilling::{spill, spill_all};

use l2::*;
use std::collections::HashSet;

pub use crate::regalloc::coloring::color_graph;
pub use crate::regalloc::interference::build_interference;
pub use crate::regalloc::spilling::spill_with_display;

fn assign_colors(func: &mut Function, coloring: &ColoringResult) {
    for block in &mut func.basic_blocks {
        for inst in &mut block.instructions {
            for var in inst
                .defs()
                .into_iter()
                .chain(inst.uses())
                .filter(|def| matches!(def, Value::Variable(_)))
            {
                if let Some(&color) = coloring.mapping.get(
                    &coloring
                        .interner
                        .get(&var)
                        .expect("all variables should be interned"),
                ) {
                    inst.replace_value(&var, coloring.interner.resolve(color));
                }
            }
        }
    }
}

pub fn allocate_registers(func: &mut Function, interner: &mut Interner<String>) {
    let original_func = func.clone();
    let prefix = "S";
    let mut suffix = 0;
    let mut all_spilled = HashSet::new();

    loop {
        let liveness = compute_liveness(func);
        let interference = build_interference(func, &liveness);
        let coloring = color_graph(&interference);

        if coloring.spilled.is_empty() {
            assign_colors(func, &coloring);
            break;
        }

        let to_spill: Vec<Value> = coloring
            .spilled
            .iter()
            .filter_map(|&var| {
                let var = coloring.interner.resolve(var);
                (!all_spilled.contains(var)).then_some(var.clone())
            })
            .collect();

        if !to_spill.is_empty() {
            for var in to_spill {
                let spilled = spill(func, &var, prefix, &mut suffix, interner);
                all_spilled.extend(spilled.into_iter());
            }
        } else {
            *func = original_func.clone();
            suffix = 0;
            spill_all(func, prefix, &mut suffix, interner);
        }
    }
}
