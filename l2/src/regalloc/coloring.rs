use crate::bitvector::BitVector;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct ColoringResult<'a> {
    pub interner: &'a Interner<Value>,
    pub mapping: HashMap<usize, usize>,
    pub spilled: Vec<usize>,
}

impl DisplayResolved for ColoringResult<'_> {
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

impl fmt::Display for ColoringResult<'_> {
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

fn gp_registers(interner: &Interner<Value>) -> Vec<usize> {
    Register::GP_REGISTERS
        .iter()
        .map(|&reg| {
            interner
                .get(&Value::Register(reg))
                .expect("registers should be interned")
        })
        .collect()
}

fn simplify(interference: &InterferenceGraph) -> Vec<usize> {
    let gp_registers = gp_registers(&interference.interner);
    let num_gp_registers = gp_registers.len();

    let mut stack = Vec::new();
    let mut worklist = BitVector::with_len(interference.graph.len());
    worklist.set_all();
    worklist.reset_from(gp_registers.iter().copied());

    while worklist.any() {
        let remaining_degrees: Vec<(usize, usize)> = worklist
            .iter()
            .map(|u| {
                let degree = interference.graph[u]
                    .iter()
                    .filter(|&v| worklist.test(v) || gp_registers.contains(&v))
                    .count();
                (u, degree)
            })
            .collect();

        let removed_node = remaining_degrees
            .iter()
            .filter(|&&(_, degree)| degree < num_gp_registers)
            .max_by_key(|&&(_, degree)| degree)
            .or_else(|| remaining_degrees.iter().max_by_key(|&&(_, degree)| degree))
            .map(|&(node, _)| node)
            .expect("graph should not be empty");

        worklist.reset(removed_node);
        stack.push(removed_node);
    }

    stack
}

fn select<'a>(interference: &'a InterferenceGraph, mut stack: Vec<usize>) -> ColoringResult<'a> {
    let gp_registers = gp_registers(&interference.interner);
    let num_gp_registers = gp_registers.len();

    let mut mapping: HashMap<usize, usize> = gp_registers.iter().map(|&reg| (reg, reg)).collect();
    let mut spilled: Vec<usize> = Vec::new();
    let mut adjacent_colors = BitVector::with_len(num_gp_registers);

    while let Some(u) = stack.pop() {
        adjacent_colors.set_from(interference.graph[u].iter().filter_map(|v| {
            mapping
                .get(&v)
                .and_then(|&color| gp_registers.iter().position(|&reg| reg == color))
        }));

        if let Some((_, &color)) = gp_registers
            .iter()
            .enumerate()
            .find(|&(index, _)| !adjacent_colors.test(index))
        {
            mapping.insert(u, color);
        } else {
            spilled.push(u);
        }

        adjacent_colors.reset_all();
    }

    ColoringResult {
        interner: &interference.interner,
        mapping,
        spilled,
    }
}

pub fn color_graph<'a>(interference: &'a InterferenceGraph) -> ColoringResult<'a> {
    select(interference, simplify(interference))
}
