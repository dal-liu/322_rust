use crate::bitvector::BitVector;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter;

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

#[derive(Debug)]
struct ColoringAllocator<'a> {
    interference: &'a InterferenceGraph<'a>,
    degree: Vec<usize>,
    move_list: Vec<HashSet<(usize, usize)>>,
    alias: Vec<Option<usize>>,
    color: Vec<Option<usize>>,

    precolored: Vec<usize>,
    initial: BTreeSet<usize>,
    simplify_worklist: BTreeSet<usize>,
    freeze_worklist: HashSet<usize>,
    spill_worklist: HashSet<usize>,
    spill_nodes: HashSet<usize>,
    coalesced_nodes: HashSet<usize>,
    colored_nodes: HashSet<usize>,
    select_stack: Vec<usize>,

    coalesced_moves: HashSet<(usize, usize)>,
    constrained_moves: HashSet<(usize, usize)>,
    frozen_moves: HashSet<(usize, usize)>,
    worklist_moves: HashSet<(usize, usize)>,
    active_moves: HashSet<(usize, usize)>,
}

impl<'a> ColoringAllocator<'a> {
    pub fn new(func: &Function, interference: &'a InterferenceGraph) -> Self {
        let num_gp_variables = interference.graph.len();

        let degree: Vec<usize> = interference
            .graph
            .iter()
            .map(|neighbors| neighbors.count())
            .collect();

        let mut move_list = vec![HashSet::new(); num_gp_variables];
        let mut worklist_moves = HashSet::new();

        func.basic_blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|inst| match inst {
                Instruction::Assign { dst, src }
                    if dst.is_gp_variable() && src.is_gp_variable() =>
                {
                    let u = interference
                        .interner
                        .get(dst)
                        .expect("registers and variables should be interned");
                    let v = interference
                        .interner
                        .get(src)
                        .expect("registers and variables should be interned");
                    Some((u, v))
                }
                _ => None,
            })
            .for_each(|(u, v)| {
                move_list[u].insert((u, v));
                move_list[v].insert((u, v));
                worklist_moves.insert((u, v));
            });

        let precolored: Vec<usize> = Register::GP_REGISTERS
            .iter()
            .map(|&reg| {
                interference
                    .interner
                    .get(&Value::Register(reg))
                    .expect("registers should be interned")
            })
            .collect();

        let color = (0..num_gp_variables)
            .map(|i| precolored.contains(&i).then_some(i))
            .collect();

        let mut initial = BTreeSet::new();
        initial.extend((0..num_gp_variables).filter(|node| !precolored.contains(node)));

        Self {
            interference,
            degree,
            move_list,
            alias: vec![None; num_gp_variables],
            color,

            precolored,
            initial,
            simplify_worklist: BTreeSet::new(),
            freeze_worklist: HashSet::new(),
            spill_worklist: HashSet::new(),
            spill_nodes: HashSet::new(),
            coalesced_nodes: HashSet::new(),
            colored_nodes: HashSet::new(),
            select_stack: Vec::new(),

            coalesced_moves: HashSet::new(),
            constrained_moves: HashSet::new(),
            frozen_moves: HashSet::new(),
            worklist_moves,
            active_moves: HashSet::new(),
        }
    }

    pub fn finish(mut self) -> ColoringResult<'a> {
        while !self.is_done() {
            if !self.simplify_worklist.is_empty() {
                self.simplify();
            } else if !self.worklist_moves.is_empty() {
                self.coalesce();
            } else if !self.freeze_worklist.is_empty() {
                self.freeze();
            } else if !self.spill_worklist.is_empty() {
                self.select_spill();
            }
        }

        todo!()
    }

    fn is_done(&self) -> bool {
        self.simplify_worklist.is_empty()
            && self.worklist_moves.is_empty()
            && self.freeze_worklist.is_empty()
            && self.spill_worklist.is_empty()
    }

    fn adjacent(&self, node: usize) -> Vec<usize> {
        let select_set: HashSet<&usize> = self.select_stack.iter().collect();
        self.interference.graph[node]
            .iter()
            .filter(|neighbor| {
                !select_set.contains(neighbor) && self.coalesced_nodes.contains(neighbor)
            })
            .collect()
    }

    fn node_moves(&self, node: usize) -> Vec<(usize, usize)> {
        let active_worklist: HashSet<&(usize, usize)> =
            self.active_moves.union(&self.worklist_moves).collect();
        self.move_list[node]
            .iter()
            .filter(|move_| active_worklist.contains(move_))
            .copied()
            .collect()
    }

    fn is_move_related(&self, node: usize) -> bool {
        !self.move_list[node].is_empty()
    }

    fn make_worklist(&mut self) {
        for &node in self.initial.iter() {
            if self.degree[node] >= Register::NUM_GP_REGISTERS {
                self.spill_worklist.insert(node);
            } else if self.is_move_related(node) {
                self.freeze_worklist.insert(node);
            } else {
                self.simplify_worklist.insert(node);
            }
        }
    }

    fn simplify(&mut self) {
        if let Some(node) = self.simplify_worklist.pop_first() {
            self.select_stack.push(node);
            for neighbor in self.adjacent(node) {
                self.decrement_degree(neighbor);
            }
        }
    }

    fn decrement_degree(&mut self, node: usize) {
        let degree = self.degree[node];
        self.degree[node] -= 1;

        if degree == Register::NUM_GP_REGISTERS {
            self.enable_moves(iter::once(node).chain(self.interference.graph[node].iter()));
            self.spill_worklist.remove(&node);

            if self.is_move_related(node) {
                self.freeze_worklist.insert(node);
            } else {
                self.simplify_worklist.insert(node);
            }
        }
    }

    fn enable_moves(&mut self, iter: impl Iterator<Item = usize>) {
        for node in iter {
            for move_ in self.node_moves(node) {
                if self.active_moves.contains(&move_) {
                    self.active_moves.remove(&move_);
                    self.worklist_moves.insert(move_);
                }
            }
        }
    }

    fn coalesce(&mut self) {
        todo!()
    }

    fn freeze(&mut self) {
        todo!()
    }

    fn select_spill(&mut self) {
        todo!()
    }
}

fn simplify(interference: &InterferenceGraph) -> Vec<usize> {
    let gp_registers: HashSet<usize> = gp_registers(&interference.interner);
    let num_gp_registers = gp_registers.len();
    let num_gp_variables = interference.graph.len();

    let mut stack = Vec::new();
    let mut worklist = BitVector::new(num_gp_variables);
    worklist.set_from((0..num_gp_variables).filter(|i| !gp_registers.contains(i)));

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
            .filter(|&&(_, k)| k < num_gp_registers)
            .max_by_key(|&&(_, k)| k)
            .or_else(|| remaining_degrees.iter().max_by_key(|&&(_, k)| k))
            .map(|&(node, _)| node)
            .expect("graph should not be empty");

        worklist.reset(removed_node);
        stack.push(removed_node);
    }

    stack
}

fn select<'a>(interference: &'a InterferenceGraph, mut stack: Vec<usize>) -> ColoringResult<'a> {
    let gp_registers: Vec<usize> = gp_registers(&interference.interner);
    let num_gp_registers = gp_registers.len();

    let mut mapping: HashMap<usize, usize> = gp_registers.iter().map(|&reg| (reg, reg)).collect();
    let mut spilled: Vec<usize> = Vec::new();
    let mut neighbor_colors = BitVector::new(num_gp_registers);

    while let Some(u) = stack.pop() {
        neighbor_colors.set_from(interference.graph[u].iter().filter_map(|v| {
            mapping
                .get(&v)
                .and_then(|&color| gp_registers.iter().position(|&reg| reg == color))
        }));

        if let Some((_, &color)) = gp_registers
            .iter()
            .enumerate()
            .find(|&(i, _)| !neighbor_colors.test(i))
        {
            mapping.insert(u, color);
        } else {
            spilled.push(u);
        }

        neighbor_colors.reset_all();
    }

    ColoringResult {
        interner: &interference.interner,
        mapping,
        spilled,
    }
}

fn gp_registers<T: FromIterator<usize>>(interner: &Interner<Value>) -> T {
    Register::GP_REGISTERS
        .iter()
        .map(|&reg| {
            interner
                .get(&Value::Register(reg))
                .expect("registers should be interned")
        })
        .collect()
}

pub fn color_graph<'a>(interference: &'a InterferenceGraph) -> ColoringResult<'a> {
    select(interference, simplify(interference))
}
