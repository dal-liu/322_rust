use crate::bitvector::BitVector;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter;

#[derive(Debug)]
pub struct ColoringResult<'a> {
    pub interner: &'a Interner<Value>,
    pub color: HashMap<usize, usize>,
    pub spill_nodes: BTreeSet<usize>,
}

#[derive(Debug)]
struct ColoringAllocator<'a, 'b> {
    interner: Interner<Instruction>,
    interference: &'a mut InterferenceGraph<'a>,
    prev_spilled: &'b HashSet<Value>,

    precolored: Vec<usize>,
    simplify_worklist: BitVector,
    freeze_worklist: BitVector,
    spill_worklist: BitVector,
    spill_nodes: BTreeSet<usize>,
    coalesced_nodes: BitVector,
    colored_nodes: BitVector,
    select_stack: Vec<usize>,

    coalesced_moves: BitVector,
    constrained_moves: BitVector,
    frozen_moves: BitVector,
    worklist_moves: BitVector,
    active_moves: BitVector,

    move_list: Vec<BitVector>,
    alias: Vec<usize>,
    color: HashMap<usize, usize>,
}

impl<'a, 'b> ColoringAllocator<'a, 'b> {
    pub fn new(
        func: &Function,
        interference: &'a mut InterferenceGraph<'a>,
        prev_spilled: &'b HashSet<Value>,
    ) -> Self {
        let mut instruction_interner = Interner::new();

        func.basic_blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .for_each(|inst| match inst {
                Instruction::Assign { dst, src }
                    if dst.is_gp_variable() && src.is_gp_variable() =>
                {
                    instruction_interner.intern(inst.clone());
                }
                _ => (),
            });

        let precolored: Vec<usize> = Register::GP_REGISTERS
            .iter()
            .map(|&reg| interference.interner[&Value::Register(reg)])
            .collect();

        let num_nodes = interference.interner.len();
        let num_moves = instruction_interner.len();
        let mut worklist_moves = BitVector::new(num_moves);
        let mut move_list = vec![BitVector::new(num_moves); num_nodes];

        func.basic_blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .for_each(|inst| match inst {
                Instruction::Assign { dst, src }
                    if dst.is_gp_variable() && src.is_gp_variable() =>
                {
                    let move_ = instruction_interner[inst];
                    worklist_moves.set(move_);

                    for var in [dst, src] {
                        let node = interference.interner[var];
                        move_list[node].set(move_);
                    }
                }
                _ => (),
            });

        let alias = (0..num_nodes).collect();

        let color = (0..num_nodes)
            .filter_map(|n| precolored.contains(&n).then_some(n))
            .map(|n| (n, n))
            .collect();

        let mut allocator = Self {
            interner: instruction_interner,
            interference,
            prev_spilled,

            precolored,
            simplify_worklist: BitVector::new(num_nodes),
            freeze_worklist: BitVector::new(num_nodes),
            spill_worklist: BitVector::new(num_nodes),
            spill_nodes: BTreeSet::new(),
            coalesced_nodes: BitVector::new(num_nodes),
            colored_nodes: BitVector::new(num_nodes),
            select_stack: Vec::new(),

            coalesced_moves: BitVector::new(num_moves),
            constrained_moves: BitVector::new(num_moves),
            frozen_moves: BitVector::new(num_moves),
            worklist_moves,
            active_moves: BitVector::new(num_moves),

            move_list,
            alias,
            color,
        };

        for node in (0..num_nodes).filter(|n| !allocator.precolored.contains(n)) {
            if allocator.interference.degree(node) >= Register::NUM_GP_REGISTERS {
                allocator.spill_worklist.set(node);
            } else if allocator.is_move_related(node) {
                allocator.freeze_worklist.set(node);
            } else {
                allocator.simplify_worklist.set(node);
            }
        }

        allocator
    }

    pub fn allocate(&mut self) {
        while self.simplify_worklist.any()
            || self.worklist_moves.any()
            || self.freeze_worklist.any()
            || self.spill_worklist.any()
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
    }

    pub fn assign_colors(mut self) -> ColoringResult<'a> {
        while let Some(u) = self.select_stack.pop() {
            let mut ok_colors = self.precolored.clone();
            let mut colored = self.colored_nodes.clone();
            colored.set_from(self.precolored.iter().copied());

            for v in &self.interference.graph[u] {
                if colored.test(self.get_alias(v)) {
                    ok_colors.retain(|&color| color != self.color[&self.get_alias(v)]);
                }
            }

            if ok_colors.is_empty() {
                self.spill_nodes.insert(u);
            } else {
                self.colored_nodes.set(u);
                self.color.insert(u, ok_colors[0]);
            }
        }

        for node in &self.coalesced_nodes {
            self.color.insert(node, self.color[&self.get_alias(node)]);
        }

        ColoringResult {
            interner: self.interference.interner,
            color: self.color,
            spill_nodes: self.spill_nodes,
        }
    }

    fn add_edge(&mut self, u: usize, v: usize) {
        if !self.interference.has_edge(u, v) && u != v {
            self.interference.add_edge(u, v);
        }
    }

    fn adjacent(&self, node: usize) -> Vec<usize> {
        let mut select = BitVector::new(self.interference.graph.len());
        select.set_from(self.select_stack.iter().copied());
        select.union(&self.coalesced_nodes);

        let mut adjacent = self.interference.graph[node].clone();
        adjacent.difference(&select);
        adjacent.iter().collect()
    }

    fn node_moves(&self, node: usize) -> Vec<usize> {
        let mut node_moves = self.active_moves.clone();
        node_moves.union(&self.worklist_moves);
        node_moves.intersection(&self.move_list[node]);
        node_moves.iter().collect()
    }

    fn is_move_related(&self, node: usize) -> bool {
        self.move_list[node].any()
    }

    fn simplify(&mut self) {
        if let Some(node) = self.simplify_worklist.iter().next() {
            self.simplify_worklist.reset(node);
            self.select_stack.push(node);

            for neighbor in self.adjacent(node) {
                self.decrement_degree(neighbor);
            }
        }
    }

    fn decrement_degree(&mut self, node: usize) {
        if self.interference.degree(node) == Register::NUM_GP_REGISTERS {
            self.enable_moves(
                iter::once(node)
                    .chain(&self.interference.graph[node])
                    .collect(),
            );
            self.spill_worklist.reset(node);

            if self.is_move_related(node) {
                self.freeze_worklist.set(node);
            } else {
                self.simplify_worklist.set(node);
            }
        }
    }

    fn enable_moves(&mut self, nodes: Vec<usize>) {
        for node in nodes {
            for move_ in self.node_moves(node) {
                if self.active_moves.test(move_) {
                    self.active_moves.reset(move_);
                    self.worklist_moves.set(move_);
                }
            }
        }
    }

    fn coalesce(&mut self) {
        if let Some(move_) = self.worklist_moves.iter().next() {
            let value_interner = self.interference.interner;

            if let Instruction::Assign { dst, src } = self.interner.resolve(move_) {
                let x = self.get_alias(value_interner[dst]);
                let y = self.get_alias(value_interner[src]);

                let (u, v) = if self.precolored.contains(&y) {
                    (y, x)
                } else {
                    (x, y)
                };

                self.worklist_moves.reset(move_);

                if u == v {
                    self.coalesced_moves.set(move_);
                    self.add_worklist(u);
                } else if self.precolored.contains(&v) || self.interference.has_edge(u, v) {
                    self.constrained_moves.set(move_);
                    self.add_worklist(u);
                    self.add_worklist(v);
                } else if (self.precolored.contains(&u) && self.can_coalesce_george(u, v))
                    || (!self.precolored.contains(&u) && self.can_coalesce_briggs(u, v))
                {
                    self.coalesced_moves.set(move_);
                    self.combine(u, v);
                    self.add_worklist(u);
                } else {
                    self.active_moves.set(move_);
                }
            }
        }
    }

    fn add_worklist(&mut self, node: usize) {
        if !self.precolored.contains(&node)
            && !self.is_move_related(node)
            && self.interference.degree(node) < Register::NUM_GP_REGISTERS
        {
            self.freeze_worklist.reset(node);
            self.simplify_worklist.set(node);
        }
    }

    fn can_coalesce_george(&self, u: usize, v: usize) -> bool {
        self.adjacent(v).iter().all(|&n| {
            self.interference.degree(n) < Register::NUM_GP_REGISTERS
                || self.precolored.contains(&n)
                || self.interference.has_edge(u, n)
        })
    }

    fn can_coalesce_briggs(&self, u: usize, v: usize) -> bool {
        let mut nodes = BitVector::new(self.interference.interner.len());
        nodes.set_from(self.adjacent(u).into_iter());
        nodes.set_from(self.adjacent(v).into_iter());

        let mut k = 0;
        for node in &nodes {
            if self.interference.degree(node) >= Register::NUM_GP_REGISTERS {
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

        let move_list = self.move_list[v].clone();
        self.move_list[u].union(&move_list);

        for neighbor in self.adjacent(v) {
            self.add_edge(neighbor, u);
            self.decrement_degree(neighbor);
        }

        if self.interference.degree(u) >= Register::NUM_GP_REGISTERS && self.freeze_worklist.test(u)
        {
            self.freeze_worklist.reset(u);
            self.spill_worklist.set(u);
        }
    }

    fn freeze(&mut self) {
        if let Some(node) = self.freeze_worklist.iter().next() {
            self.freeze_worklist.reset(node);
            self.simplify_worklist.set(node);
            self.freeze_moves(node);
        }
    }

    fn freeze_moves(&mut self, u: usize) {
        let value_interner = self.interference.interner;

        for move_ in self.node_moves(u) {
            if self.active_moves.test(move_) {
                self.active_moves.reset(move_);
            } else {
                self.worklist_moves.reset(move_);
            }

            self.frozen_moves.set(move_);

            let v = match self.interner.resolve(move_) {
                Instruction::Assign { dst, src } => {
                    value_interner[if value_interner[dst] == u { src } else { dst }]
                }
                _ => panic!("not a move"),
            };

            if self.node_moves(v).is_empty()
                && self.interference.degree(v) < Register::NUM_GP_REGISTERS
            {
                self.freeze_worklist.reset(v);
                self.simplify_worklist.set(v);
            }
        }
    }

    fn select_spill(&mut self) {
        // TODO: implement loop heuristic
        if let Some(node) = self.spill_worklist.iter().find(|&n| {
            !self
                .prev_spilled
                .contains(self.interference.interner.resolve(n))
        }) {
            self.spill_worklist.reset(node);
            self.simplify_worklist.set(node);
            self.freeze_moves(node);
        } else if let Some(node) = self.spill_worklist.iter().next() {
            self.spill_worklist.reset(node);
            self.simplify_worklist.set(node);
            self.freeze_moves(node);
        }
    }
}

pub fn color_graph<'a, 'b>(
    func: &Function,
    interference: &'a mut InterferenceGraph<'a>,
    prev_spilled: &'b HashSet<Value>,
) -> ColoringResult<'a> {
    let mut allocator = ColoringAllocator::new(func, interference, prev_spilled);
    allocator.allocate();
    allocator.assign_colors()
}
