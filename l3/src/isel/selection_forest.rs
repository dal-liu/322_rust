use l3::*;
use std::fmt;
use utils::DisplayResolved;

use crate::analysis::{DefUseChain, LivenessResult};
use crate::isel::contexts::Context;

type NodeId = usize;

#[derive(Debug, PartialEq, Eq)]
pub enum OpKind {
    Assign,
    Add,
    Sub,
    Mul,
    BitAnd,
    Shl,
    Shr,
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    Load,
    Store,
    Ret,
    Br,
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use OpKind::*;

        let op = match self {
            Assign => "<-",
            Add => "+",
            Sub => "-",
            Mul => "*",
            BitAnd => "&",
            Shl => "<<",
            Shr => ">>",
            Lt => "<",
            Le => "<=",
            Eq => "=",
            Ge => ">=",
            Gt => ">",
            Load => "load",
            Store => "store",
            Ret => "ret",
            Br => "br",
        };

        write!(f, "{}", op)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeKind {
    Op(OpKind),
    Number(i64),
    Label(SymbolId),
    Function(SymbolId),
    Variable(SymbolId),
}

#[derive(Debug)]
pub struct SFNode {
    pub kind: NodeKind,
    pub children: Vec<NodeId>,
}

impl DisplayResolved for SFNode {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter,
        interner: &utils::Interner<String>,
    ) -> std::fmt::Result {
        match &self.kind {
            NodeKind::Op(op) => write!(f, "{}", op),
            NodeKind::Number(num) => write!(f, "{}", num),
            NodeKind::Label(label) => write!(f, ":{}", interner.resolve(label.0)),
            NodeKind::Function(callee) => write!(f, "@{}", interner.resolve(callee.0)),
            NodeKind::Variable(var) => write!(f, "%{}", interner.resolve(var.0)),
        }
    }
}

#[derive(Debug)]
pub struct SelectionForest {
    pub arena: Vec<SFNode>,
    pub roots: Vec<NodeId>,
}

impl SelectionForest {
    pub fn new(
        func: &Function,
        liveness: &LivenessResult,
        def_use: &DefUseChain,
        ctx: &mut Context,
    ) -> Self {
        use Instruction::*;

        let mut forest = Self {
            arena: Vec::new(),
            roots: Vec::new(),
        };

        for &id in &ctx.inst_ids {
            let root = match &func.basic_blocks[ctx.block_id.0].instructions[id] {
                Assign { dst, src } => {
                    let src_node = forest.alloc_value(*src, vec![]);
                    let assign_node = forest.alloc_op(OpKind::Assign, vec![src_node]);
                    forest.alloc_variable(*dst, vec![assign_node])
                }
                Binary { dst, lhs, op, rhs } => {
                    let lhs_node = forest.alloc_value(*lhs, vec![]);
                    let rhs_node = forest.alloc_value(*rhs, vec![]);

                    let op_kind = match op {
                        BinaryOp::Add => OpKind::Add,
                        BinaryOp::Sub => OpKind::Sub,
                        BinaryOp::Mul => OpKind::Mul,
                        BinaryOp::BitAnd => OpKind::BitAnd,
                        BinaryOp::Shl => OpKind::Shl,
                        BinaryOp::Shr => OpKind::Shr,
                    };

                    let op_node = forest.alloc_op(op_kind, vec![lhs_node, rhs_node]);
                    forest.alloc_variable(*dst, vec![op_node])
                }
                Compare { dst, lhs, cmp, rhs } => {
                    let lhs_node = forest.alloc_value(*lhs, vec![]);
                    let rhs_node = forest.alloc_value(*rhs, vec![]);

                    let cmp_kind = match cmp {
                        CompareOp::Lt => OpKind::Lt,
                        CompareOp::Le => OpKind::Le,
                        CompareOp::Eq => OpKind::Eq,
                        CompareOp::Ge => OpKind::Ge,
                        CompareOp::Gt => OpKind::Gt,
                    };

                    let cmp_node = forest.alloc_op(cmp_kind, vec![lhs_node, rhs_node]);
                    forest.alloc_variable(*dst, vec![cmp_node])
                }
                Load { dst, src } => {
                    let src_node = forest.alloc_variable(*src, vec![]);
                    let load_node = forest.alloc_op(OpKind::Load, vec![src_node]);
                    forest.alloc_variable(*dst, vec![load_node])
                }
                Store { dst, src } => {
                    let dst_node = forest.alloc_variable(*dst, vec![]);
                    let src_node = forest.alloc_value(*src, vec![]);
                    forest.alloc_op(OpKind::Store, vec![dst_node, src_node])
                }
                Return => forest.alloc_op(OpKind::Ret, vec![]),
                ReturnValue(val) => {
                    let val_node = forest.alloc_value(*val, vec![]);
                    forest.alloc_op(OpKind::Ret, vec![val_node])
                }
                Branch(_) => forest.alloc_op(OpKind::Br, vec![]),
                BranchCond { cond, .. } => {
                    let cond_node = forest.alloc_value(*cond, vec![]);
                    forest.alloc_op(OpKind::Br, vec![cond_node])
                }
                Label(_) | Call { .. } | CallResult { .. } => panic!("illegal context instruction"),
            };

            forest.roots.push(root);
        }

        'outer: loop {
            for i in 0..forest.roots.len() - 1 {
                for j in i + 1..forest.roots.len() {
                    if forest.try_merge(func, ctx, liveness, def_use, i, j) {
                        continue 'outer;
                    }
                }
            }
            break;
        }

        forest
    }

    fn alloc_op(&mut self, op: OpKind, children: Vec<NodeId>) -> NodeId {
        self.alloc(SFNode {
            kind: NodeKind::Op(op),
            children,
        })
    }

    fn alloc_value(&mut self, val: Value, children: Vec<NodeId>) -> NodeId {
        let kind = match val {
            Value::Number(num) => NodeKind::Number(num),
            Value::Label(label) => NodeKind::Label(label),
            Value::Function(callee) => NodeKind::Function(callee),
            Value::Variable(var) => NodeKind::Variable(var),
        };
        self.alloc(SFNode { kind, children })
    }

    fn alloc_variable(&mut self, var: SymbolId, children: Vec<NodeId>) -> NodeId {
        self.alloc(SFNode {
            kind: NodeKind::Variable(var),
            children,
        })
    }

    fn alloc(&mut self, node: SFNode) -> NodeId {
        let id = self.arena.len();
        self.arena.push(node);
        id
    }

    fn leaves(&self, root: NodeId) -> Vec<NodeId> {
        let mut leaves = Vec::new();
        let mut stack = vec![root];

        while let Some(id) = stack.pop() {
            let node = &self.arena[id];
            if node.children.is_empty() {
                leaves.push(id);
            } else {
                stack.extend(node.children.iter().rev());
            }
        }

        leaves
    }

    fn try_merge(
        &mut self,
        func: &Function,
        ctx: &mut Context,
        liveness: &LivenessResult,
        def_use: &DefUseChain,
        first: usize,
        second: usize,
    ) -> bool {
        let u = self.roots[first];
        let v = self.roots[second];

        for leaf in self.leaves(v) {
            if self.arena[u].kind != self.arena[leaf].kind {
                continue;
            }

            let block = &func.basic_blocks[ctx.block_id.0];
            let inst1 = &block.instructions[ctx.inst_ids[first]];
            let inst2 = &block.instructions[ctx.inst_ids[second]];

            if let NodeKind::Variable(var) = self.arena[u].kind {
                if !liveness.is_dead_at(ctx.block_id, ctx.inst_ids[second], var)
                    || def_use.users_of(inst1).as_slice() != [inst2]
                {
                    return false;
                }
            } else {
                return false;
            }

            for i in first + 1..second {
                let mid = &block.instructions[ctx.inst_ids[i]];

                if matches!(inst1, Instruction::Load { .. }) {
                    match mid {
                        Instruction::Load { .. } | Instruction::Store { .. } => return false,
                        _ => (),
                    }
                } else if mid.uses().iter().any(|use_| inst1.defs() == Some(*use_))
                    || inst1.uses().iter().any(|use_| mid.defs() == Some(*use_))
                {
                    return false;
                }
            }

            let (a, b) = if u < leaf {
                let (left, right) = self.arena.split_at_mut(leaf);
                (&mut right[0], &left[u])
            } else {
                let (left, right) = self.arena.split_at_mut(u);
                (&mut left[leaf], &right[0])
            };

            a.children.extend(b.children.iter());
            self.roots.remove(first);
            ctx.inst_ids.remove(first);

            return true;
        }

        false
    }
}

impl DisplayResolved for SelectionForest {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter,
        interner: &utils::Interner<String>,
    ) -> std::fmt::Result {
        for &root in &self.roots {
            let mut stack = vec![(root, 0)];
            while let Some((id, indent)) = stack.pop() {
                let node = &self.arena[id];
                writeln!(f, "{}{}", "  ".repeat(indent), node.resolved(interner))?;
                if !node.children.is_empty() {
                    stack.extend(node.children.iter().rev().map(|&child| (child, indent + 1)));
                }
            }
        }
        Ok(())
    }
}

pub fn generate_forests(
    func: &Function,
    liveness: &LivenessResult,
    def_use: &DefUseChain,
    contexts: &mut [Context],
) -> Vec<SelectionForest> {
    contexts
        .iter_mut()
        .map(|ctx| SelectionForest::new(func, liveness, def_use, ctx))
        .collect()
}
