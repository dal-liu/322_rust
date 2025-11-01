mod coloring;
mod interference;
mod spilling;

use crate::analysis::compute_liveness;
use crate::regalloc::coloring::ColoringResult;
use crate::regalloc::spilling::{spill, spill_all};

use l2::*;
use std::collections::HashSet;

pub use crate::regalloc::coloring::color_graph;
pub use crate::regalloc::interference::InterferenceGraph;
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

pub fn allocate_registers(func: &Function) -> Function {
    let mut new_func = func.clone();
    let mut all_spilled = HashSet::new();

    loop {
        let liveness = compute_liveness(func);
        let interference = InterferenceGraph::build(func, &liveness);
        let coloring = color_graph(&interference);

        if coloring.spilled.is_empty() {
            assign_colors(&mut new_func, &coloring);
            break;
        }

        if coloring.spilled.iter().all(|var| all_spilled.contains(var)) {
            new_func = func.clone();
            spill_all(&mut new_func, "S");
            break;
        }

        for &var in &coloring.spilled {
            if !all_spilled.contains(&var) {
                spill(&mut new_func, &Value::Variable(SymbolId(var)), "S");
                all_spilled.insert(var);
            }
        }
    }

    new_func
}
