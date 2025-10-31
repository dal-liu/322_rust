use crate::bitvector::BitVector;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct ColoringResult {
    pub interner: Interner<Value>,
    pub mapping: HashMap<usize, usize>,
    pub spilled: Vec<usize>,
}

impl DisplayResolved for ColoringResult {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter,
        interner: &Interner<String>,
    ) -> std::fmt::Result {
        for (&var, &reg) in &self.mapping {
            writeln!(
                f,
                "{} {}",
                self.interner.resolve(var).resolved(interner),
                self.interner.resolve(reg).resolved(interner)
            )?;
        }

        for &var in &self.spilled {
            writeln!(f, "{}", self.interner.resolve(var).resolved(interner))?;
        }

        Ok(())
    }
}

impl fmt::Display for ColoringResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (&var, &reg) in &self.mapping {
            writeln!(f, "{} {}", var, reg,)?;
        }

        for &var in &self.spilled {
            writeln!(f, "{}", var)?;
        }

        Ok(())
    }
}

fn simplify(interference: &InterferenceGraph) -> Vec<usize> {
    let num_gp_registers = Register::GP_REGISTERS.len();
    let mut stack = Vec::new();
    let mut worklist = BitVector::with_len(interference.graph.len());
    worklist.set_all();

    while worklist.any() {
        let remaining_degrees: Vec<(usize, usize)> = worklist
            .iter()
            .map(|u| {
                let degree = interference.graph[u]
                    .iter()
                    .filter(|&v| worklist.test(v))
                    .count();
                (u, degree)
            })
            .collect();

        let removed_node = remaining_degrees
            .iter()
            .filter(|&&(_, k)| k < num_gp_registers)
            .max_by_key(|&&(_, k)| k)
            .or(remaining_degrees.iter().max_by_key(|&&(_, k)| k))
            .map(|&(n, _)| n)
            .expect("graph should not be empty");

        worklist.reset(removed_node);
        stack.push(removed_node);
    }

    stack
}

fn select(interference: &InterferenceGraph, mut stack: Vec<usize>) -> ColoringResult {
    let interner = interference.interner.clone();
    let gp_registers: Vec<usize> = Register::GP_REGISTERS
        .iter()
        .map(|reg| {
            interner
                .get(&Value::Register(reg.clone()))
                .expect("registers should all be interned")
        })
        .collect();

    let mut mapping: HashMap<usize, usize> = gp_registers.iter().map(|&reg| (reg, reg)).collect();
    let mut spilled: Vec<usize> = Vec::new();
    let mut colored = BitVector::with_len(interference.graph.len());
    let mut adjacent_colors = BitVector::with_len(Register::GP_REGISTERS.len());

    while let Some(node) = stack.pop() {
        colored.set(node);

        if matches!(interner.resolve(node), Value::Register(_)) {
            continue;
        }

        adjacent_colors.set_from(interference.graph[node].iter().filter_map(|n| {
            colored.test(n).then_some(n).and_then(|n| {
                mapping
                    .get(&n)
                    .and_then(|&color| gp_registers.iter().position(|&reg| reg == color))
            })
        }));

        if let Some((_, color)) = gp_registers
            .iter()
            .enumerate()
            .find(|(i, _)| !adjacent_colors.test(*i))
        {
            mapping.insert(node, *color);
        } else {
            spilled.push(node);
        }

        adjacent_colors.reset_all();
    }

    ColoringResult {
        interner,
        mapping,
        spilled,
    }
}

pub fn color_graph(interference: InterferenceGraph) -> ColoringResult {
    let stack = simplify(&interference);
    let result = select(&interference, stack);
    result
}
