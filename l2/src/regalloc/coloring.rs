use crate::analysis::LivenessResult;
use crate::bitvector::BitVector;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;
use std::collections::{HashMap, HashSet};
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
    interner: Interner<(usize, usize)>,
    degree: Vec<usize>,
    move_list: Vec<BitVector>,
    alias: Vec<usize>,
    color: Vec<Option<usize>>,

    precolored: Vec<usize>,
    initial: BitVector,
    simplify_worklist: BitVector,
    freeze_worklist: BitVector,
    spill_worklist: BitVector,
    spill_nodes: BitVector,
    coalesced_nodes: BitVector,
    colored_nodes: BitVector,
    select_stack: Vec<usize>,

    coalesced_moves: BitVector,
    constrained_moves: BitVector,
    frozen_moves: BitVector,
    worklist_moves: BitVector,
    active_moves: BitVector,
}

impl<'a> ColoringAllocator<'a> {
    pub fn new(func: &Function, interference: &'a InterferenceGraph) -> Self {
        let moves: Vec<(usize, usize)> = func
            .basic_blocks
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

                    if matches!(dst, Value::Register(_)) && matches!(src, Value::Register(_)) {
                        None
                    } else if matches!(src, Value::Register(_)) {
                        Some((v, u))
                    } else {
                        Some((u, v))
                    }
                }
                _ => None,
            })
            .collect();

        let num_gp_variables = interference.interner.len();
        let num_moves = moves.len();

        let mut interner = Interner::new();
        let mut move_list = vec![BitVector::new(num_moves); num_gp_variables];
        let mut worklist_moves = BitVector::new(num_moves);

        for (u, v) in moves {
            let index = interner.intern((u, v));
            move_list[u].set(index);
            move_list[v].set(index);
            worklist_moves.set(index);
        }

        let degree: Vec<usize> = interference
            .graph
            .iter()
            .map(|neighbors| neighbors.count())
            .collect();

        let alias = (0..num_gp_variables).collect();

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

        let mut initial = BitVector::new(num_gp_variables);
        initial.set_from((0..num_gp_variables).filter(|node| !precolored.contains(node)));

        Self {
            interference,
            interner,
            degree,
            move_list,
            alias,
            color,

            precolored,
            initial,
            simplify_worklist: BitVector::new(num_gp_variables),
            freeze_worklist: BitVector::new(num_gp_variables),
            spill_worklist: BitVector::new(num_gp_variables),
            spill_nodes: BitVector::new(num_gp_variables),
            coalesced_nodes: BitVector::new(num_gp_variables),
            colored_nodes: BitVector::new(num_gp_variables),
            select_stack: Vec::new(),

            coalesced_moves: BitVector::new(num_moves),
            constrained_moves: BitVector::new(num_moves),
            frozen_moves: BitVector::new(num_moves),
            worklist_moves,
            active_moves: BitVector::new(num_moves),
        }
    }

    pub fn finish(mut self) -> ColoringResult<'a> {
        while self.simplify_worklist.any()
            && self.worklist_moves.any()
            && self.freeze_worklist.any()
            && self.spill_worklist.any()
        {
            if self.simplify_worklist.any() {
                self.simplify();
            } else if self.worklist_moves.any() {
                self.coalesce();
            } else if self.freeze_worklist.any() {
                self.freeze();
            } else if self.spill_worklist.any() {
                self.select_spill();
            }
        }

        todo!()
    }

    fn adjacent(&self, node: usize) -> BitVector {
        let mut select_bitvector = BitVector::new(self.interference.graph.len());
        select_bitvector.set_from(self.select_stack.iter().copied());
        select_bitvector.union(&self.coalesced_nodes);

        let mut adjacent = self.interference.graph[node].clone();
        adjacent.difference(&select_bitvector);
        adjacent
    }

    fn node_moves(&self, node: usize) -> BitVector {
        let mut node_moves = self.active_moves.clone();
        node_moves.union(&self.worklist_moves);
        node_moves.intersection(&self.move_list[node]);
        node_moves
    }

    fn is_move_related(&self, node: usize) -> bool {
        self.move_list[node].any()
    }

    fn make_worklist(&mut self) {
        for node in &self.initial {
            if self.degree[node] >= Register::NUM_GP_REGISTERS {
                self.spill_worklist.set(node);
            } else if self.is_move_related(node) {
                self.freeze_worklist.set(node);
            } else {
                self.simplify_worklist.set(node);
            }
        }
    }

    fn simplify(&mut self) {
        if let Some(node) = self.simplify_worklist.iter().next() {
            self.simplify_worklist.reset(node);
            self.select_stack.push(node);

            for neighbor in &self.adjacent(node) {
                self.decrement_degree(neighbor);
            }
        }
    }

    fn decrement_degree(&mut self, node: usize) {
        let degree = self.degree[node];
        self.degree[node] -= 1;

        if degree == Register::NUM_GP_REGISTERS {
            self.enable_moves(iter::once(node).chain(&self.interference.graph[node]));
            self.spill_worklist.reset(node);

            if self.is_move_related(node) {
                self.freeze_worklist.set(node);
            } else {
                self.simplify_worklist.set(node);
            }
        }
    }

    fn enable_moves(&mut self, iter: impl Iterator<Item = usize>) {
        for node in iter {
            for move_ in &self.node_moves(node) {
                if self.active_moves.test(move_) {
                    self.active_moves.reset(move_);
                    self.worklist_moves.set(move_);
                }
            }
        }
    }

    fn coalesce(&mut self) {
        if let Some(move_) = self.worklist_moves.iter().next() {
            let &(a, b) = self.interner.resolve(move_);
            let x = self.get_alias(a);
            let y = self.get_alias(b);

            let (u, v) = if self.precolored.contains(&y) {
                (y, x)
            } else {
                (x, y)
            };

            let index = self
                .interner
                .get(&(u, v))
                .expect("moves should be interned");

            if u == v {
                self.coalesced_moves.set(index);
                self.add_worklist(u);
            } else if self.precolored.contains(&v) || self.interference.graph[u].test(v) {
                self.constrained_moves.set(index);
                self.add_worklist(u);
                self.add_worklist(v);
            } else if (self.precolored.contains(&u) && self.can_coalesce_george(u, v))
                || (!self.precolored.contains(&u) && self.can_coalesce_briggs(u, v))
            {
                self.coalesced_moves.set(index);
                self.combine(u, v);
                self.add_worklist(u);
            } else {
                self.active_moves.set(index);
            }
        }
    }

    fn add_worklist(&mut self, node: usize) {
        if !self.precolored.contains(&node)
            && !self.is_move_related(node)
            && self.degree[node] < Register::NUM_GP_REGISTERS
        {
            self.freeze_worklist.reset(node);
            self.simplify_worklist.set(node);
        }
    }

    fn can_coalesce_george(&self, u: usize, v: usize) -> bool {
        self.adjacent(v).iter().all(|neighbor| {
            self.degree[neighbor] < Register::NUM_GP_REGISTERS
                || self.precolored.contains(&neighbor)
                || self.interference.graph[u].test(neighbor)
        })
    }

    fn can_coalesce_briggs(&self, u: usize, v: usize) -> bool {
        let mut nodes = self.adjacent(u).clone();
        nodes.union(&self.adjacent(v));

        let mut k = 0;
        for node in &nodes {
            if self.degree[node] >= Register::NUM_GP_REGISTERS {
                k += 1;
            }
        }
        return k < Register::NUM_GP_REGISTERS;
    }

    fn get_alias(&self, node: usize) -> usize {
        if self.coalesced_nodes.test(node) {
            self.get_alias(self.alias[node])
        } else {
            node
        }
    }

    fn combine(&mut self, u: usize, v: usize) {
        if self.freeze_worklist.test(v) {
            self.freeze_worklist.reset(v);
        } else {
            self.spill_worklist.reset(v);
        }

        self.coalesced_nodes.set(v);
        self.alias[v] = u;

        let (left, right) = self.move_list.split_at_mut(v);
        left[u].union(&right[0]);

        for neighbor in &self.adjacent(v) {
            todo!()
        }
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
    let num_gp_variables = interference.interner.len();

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

        neighbor_colors.clear();
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
