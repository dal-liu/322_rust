use l3::*;
use std::fmt;
use utils::{DisplayResolved, Interner};

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Debug)]
pub struct OpSFNode {
    kind: OpKind,
    children: Vec<NodeId>,
    result: Option<SymbolId>,
}

impl DisplayResolved for OpSFNode {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        if let Some(res) = &self.result {
            write!(f, "{} {}", interner.resolve(res.0), &self.kind)
        } else {
            write!(f, "{}", &self.kind)
        }
    }
}

#[derive(Debug)]
pub struct ValueSFNode(Value);

impl DisplayResolved for ValueSFNode {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        write!(f, "{}", self.0.resolved(interner))
    }
}

#[derive(Debug)]
pub enum SFNode {
    Op(OpSFNode),
    Value(ValueSFNode),
}

impl DisplayResolved for SFNode {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        match self {
            SFNode::Op(op) => write!(f, "{}", op.resolved(interner)),
            SFNode::Value(val) => write!(f, "{}", val.resolved(interner)),
        }
    }
}

#[derive(Debug)]
pub struct SelectionForest {
    pub arena: Vec<SFNode>,
    pub roots: Vec<NodeId>,
}

impl SelectionForest {
    pub fn new(func: &Function, ctx: &mut Context) -> Self {
        use Instruction::*;

        let mut forest = Self {
            arena: Vec::new(),
            roots: Vec::new(),
        };

        for &id in &ctx.inst_ids {
            match &func.basic_blocks[ctx.block_id.0].instructions[id] {
                Assign { dst, src } => forest.make_root(OpKind::Assign, [*src], Some(*dst)),
                Binary { dst, lhs, op, rhs } => {
                    let op_kind = match op {
                        BinaryOp::Add => OpKind::Add,
                        BinaryOp::Sub => OpKind::Sub,
                        BinaryOp::Mul => OpKind::Mul,
                        BinaryOp::BitAnd => OpKind::BitAnd,
                        BinaryOp::Shl => OpKind::Shl,
                        BinaryOp::Shr => OpKind::Shr,
                    };
                    forest.make_root(op_kind, [*lhs, *rhs], Some(*dst))
                }
                Compare { dst, lhs, cmp, rhs } => {
                    let cmp_kind = match cmp {
                        CompareOp::Lt => OpKind::Lt,
                        CompareOp::Le => OpKind::Le,
                        CompareOp::Eq => OpKind::Eq,
                        CompareOp::Ge => OpKind::Ge,
                        CompareOp::Gt => OpKind::Gt,
                    };
                    forest.make_root(cmp_kind, [*lhs, *rhs], Some(*dst))
                }
                Load { dst, src } => {
                    forest.make_root(OpKind::Load, [Value::Variable(*src)], Some(*dst))
                }
                Store { dst, src } => {
                    forest.make_root(OpKind::Store, [Value::Variable(*dst), *src], None)
                }
                Return => forest.make_root(OpKind::Ret, [], None),
                ReturnValue(val) => forest.make_root(OpKind::Ret, [*val], None),
                Branch(label) => forest.make_root(OpKind::Br, [Value::Label(*label)], None),
                BranchCond { cond, label } => {
                    forest.make_root(OpKind::Br, [*cond, Value::Label(*label)], None)
                }
                Label(_) | Call { .. } | CallResult { .. } => {
                    unreachable!("illegal context instruction")
                }
            }
        }

        forest
    }

    pub fn merge_all(
        &mut self,
        func: &Function,
        ctx: &mut Context,
        liveness: &LivenessResult,
        def_use: &DefUseChain,
    ) {
        'outer: loop {
            for i in 0..self.roots.len() - 1 {
                for j in i + 1..self.roots.len() {
                    if self.try_merge(func, ctx, liveness, def_use, i, j) {
                        continue 'outer;
                    }
                }
            }
            break;
        }
    }

    fn make_root(
        &mut self,
        kind: OpKind,
        children: impl IntoIterator<Item = Value>,
        result: Option<SymbolId>,
    ) {
        let mut alloc = |node| {
            let id = self.arena.len();
            self.arena.push(node);
            id
        };

        let children = children
            .into_iter()
            .map(|val| alloc(SFNode::Value(ValueSFNode(val))))
            .collect();

        let op = alloc(SFNode::Op(OpSFNode {
            kind,
            children,
            result,
        }));

        self.roots.push(op);
    }

    fn leaves(&self, root: NodeId) -> Vec<NodeId> {
        let mut leaves = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            match &self.arena[id] {
                SFNode::Op(node) => stack.extend(node.children.iter().rev()),
                SFNode::Value(_) => leaves.push(id),
            }
        }
        leaves
    }

    fn parent(&self, child: NodeId) -> Option<NodeId> {
        for (i, node) in self.arena.iter().enumerate() {
            if let SFNode::Op(op) = node {
                if op.children.contains(&child) {
                    return Some(i);
                }
            }
        }
        None
    }

    fn try_merge(
        &mut self,
        func: &Function,
        ctx: &mut Context,
        liveness: &LivenessResult,
        def_use: &DefUseChain,
        i: usize,
        j: usize,
    ) -> bool {
        let u = self.roots[i];
        let v = self.roots[j];

        let node1 = match &self.arena[u] {
            SFNode::Op(node) => {
                if node.result.is_none() {
                    return false;
                }
                node
            }
            SFNode::Value(_) => unreachable!("root should be an op"),
        };

        'outer: for leaf in self.leaves(v) {
            let result = match &self.arena[leaf] {
                SFNode::Value(ValueSFNode(Value::Variable(var))) if Some(*var) == node1.result => {
                    *var
                }
                _ => continue,
            };

            let block = &func.basic_blocks[ctx.block_id.0];
            let inst1 = &block.instructions[ctx.inst_ids[i]];
            let inst2 = &block.instructions[ctx.inst_ids[j]];

            if !liveness.is_dead_at(ctx.block_id, ctx.inst_ids[j], result)
                || def_use.users_of(inst1).as_slice() != [inst2]
            {
                continue;
            }

            for k in i + 1..j {
                let mid = &block.instructions[ctx.inst_ids[k]];

                if matches!(inst1, Instruction::Load { .. }) {
                    if matches!(mid, Instruction::Load { .. })
                        || matches!(mid, Instruction::Store { .. })
                    {
                        continue 'outer;
                    }
                } else {
                    if mid.uses().iter().any(|use_| inst1.defs() == Some(*use_))
                        || inst1.uses().iter().any(|use_| mid.defs() == Some(*use_))
                    {
                        continue 'outer;
                    }
                }
            }

            let parent = self.parent(leaf).expect("parent of leaf should exist");

            let SFNode::Op(op) = &mut self.arena[parent] else {
                unreachable!("parent should be op node");
            };

            let index = op
                .children
                .iter()
                .position(|&child| child == leaf)
                .expect("leaf should exist in parent children");

            op.children[index] = u;
            self.roots.remove(i);
            ctx.inst_ids.remove(i);

            return true;
        }

        false
    }
}

impl DisplayResolved for SelectionForest {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        for &root in &self.roots {
            let mut stack = vec![(root, 0)];
            while let Some((id, indent)) = stack.pop() {
                let node = &self.arena[id];
                writeln!(f, "{}{}", "  ".repeat(indent), node.resolved(interner))?;
                if let SFNode::Op(op) = node {
                    stack.extend(op.children.iter().rev().map(|&child| (child, indent + 1)));
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
        .map(|ctx| {
            let mut forest = SelectionForest::new(func, ctx);
            forest.merge_all(func, ctx, liveness, def_use);
            forest
        })
        .collect()
}
