use std::fmt;

use l3::*;
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
pub enum NodeKind {
    Op {
        kind: OpKind,
        result: Option<SymbolId>,
    },
    Value(Value),
}

impl DisplayResolved for NodeKind {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        match self {
            NodeKind::Op { kind, result } => {
                let res = result
                    .and_then(|var| Some(format!("%{} ", interner.resolve(var.0))))
                    .unwrap_or_else(|| "".to_string());
                write!(f, "{}{}", res, kind)
            }
            NodeKind::Value(val) => write!(f, "{}", val.resolved(interner)),
        }
    }
}

#[derive(Debug)]
pub struct Node {
    kind: NodeKind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl DisplayResolved for Node {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        write!(f, "{}", &self.kind.resolved(interner))
    }
}

#[derive(Debug)]
pub struct SelectionForest {
    pub arena: Vec<Node>,
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

    fn alloc(&mut self, node: Node) -> NodeId {
        let id = self.arena.len();
        self.arena.push(node);
        id
    }

    fn make_root(
        &mut self,
        kind: OpKind,
        children: impl IntoIterator<Item = Value>,
        result: Option<SymbolId>,
    ) {
        let children: Vec<NodeId> = children
            .into_iter()
            .map(|val| {
                self.alloc(Node {
                    kind: NodeKind::Value(val),
                    parent: None,
                    children: Vec::new(),
                })
            })
            .collect();

        let op = self.alloc(Node {
            kind: NodeKind::Op { kind, result },
            parent: None,
            children: children.clone(),
        });

        for id in children {
            let node = &mut self.arena[id];
            node.parent = Some(op);
        }

        self.roots.push(op);
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

        let result = match &self.arena[u].kind {
            NodeKind::Op {
                result: Some(res), ..
            } => *res,
            _ => return false,
        };

        let Some(leaf) = self.matching_leaf(v, result) else {
            return false;
        };

        let block = &func.basic_blocks[ctx.block_id.0];
        let inst1 = &block.instructions[ctx.inst_ids[i]];
        let inst2 = &block.instructions[ctx.inst_ids[j]];

        if !liveness.is_dead_at(ctx.block_id, ctx.inst_ids[j], result)
            || !def_use.is_only_user(inst1, inst2)
        {
            return false;
        }

        for k in i + 1..j {
            let mid = &block.instructions[ctx.inst_ids[k]];

            match inst1 {
                Instruction::Load { .. } => {
                    if matches!(mid, Instruction::Load { .. })
                        || matches!(mid, Instruction::Store { .. })
                    {
                        return false;
                    }
                }
                _ => {
                    if mid.uses().iter().any(|use_| inst1.defs() == Some(*use_))
                        || inst1.uses().iter().any(|use_| mid.defs() == Some(*use_))
                    {
                        return false;
                    }
                }
            }
        }

        let parent = self.arena[leaf]
            .parent
            .expect("parent of leaf should exist");

        if let Some(id) = self.arena[parent]
            .children
            .iter_mut()
            .find(|&&mut child| child == leaf)
        {
            *id = u;
        } else {
            unreachable!("leaf should exist in parent children")
        }

        self.roots.remove(i);
        ctx.inst_ids.remove(i);

        true
    }

    fn matching_leaf(&self, root: NodeId, target: SymbolId) -> Option<NodeId> {
        let mut leaf = None;
        let mut stack = vec![root];

        while let Some(id) = stack.pop() {
            let node = &self.arena[id];

            if !node.children.is_empty() {
                stack.extend(node.children.iter().rev());
                continue;
            }

            if matches!(&node.kind, NodeKind::Value(Value::Variable(var)) if *var == target) {
                if leaf.is_some() {
                    return None;
                }
                leaf = Some(id);
            }
        }

        leaf
    }
}

impl DisplayResolved for SelectionForest {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
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
        .map(|ctx| {
            let mut forest = SelectionForest::new(func, ctx);
            forest.merge_all(func, ctx, liveness, def_use);
            forest
        })
        .collect()
}
