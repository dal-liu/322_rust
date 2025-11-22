use l3::*;

use crate::isel::contexts::Context;

pub type NodeId = usize;

#[derive(Debug)]
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

#[derive(Debug)]
pub enum NodeKind {
    Op(OpKind),
    Value(Value),
}

#[derive(Debug)]
pub struct SFNode {
    pub kind: NodeKind,
    pub children: Vec<NodeId>,
}

impl SFNode {
    pub fn op(op: OpKind) -> Self {
        Self {
            kind: NodeKind::Op(op),
            children: Vec::new(),
        }
    }

    pub fn value(val: Value) -> Self {
        Self {
            kind: NodeKind::Value(val),
            children: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct SelectionForest {
    pub arena: Vec<SFNode>,
    pub roots: Vec<NodeId>,
}

impl SelectionForest {
    pub fn new(ctx: &Context) -> Self {
        use Instruction::*;

        let mut forest = Self {
            arena: Vec::new(),
            roots: Vec::new(),
        };

        for inst in &ctx.instructions {
            match inst {
                Assign { dst, src } => {
                    let src_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*src),
                        children: vec![],
                    });

                    let assign_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Assign),
                        children: vec![src_node],
                    });

                    let dst_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*dst)),
                        children: vec![assign_node],
                    });

                    forest.add_root(dst_node);
                }

                Binary { dst, lhs, op, rhs } => {
                    let lhs_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*lhs),
                        children: vec![],
                    });

                    let rhs_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*rhs),
                        children: vec![],
                    });

                    let op_kind = match op {
                        BinaryOp::Add => OpKind::Add,
                        BinaryOp::Sub => OpKind::Sub,
                        BinaryOp::Mul => OpKind::Mul,
                        BinaryOp::BitAnd => OpKind::BitAnd,
                        BinaryOp::Shl => OpKind::Shl,
                        BinaryOp::Shr => OpKind::Shr,
                    };

                    let op_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(op_kind),
                        children: vec![lhs_node, rhs_node],
                    });

                    let dst_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*dst)),
                        children: vec![op_node],
                    });

                    forest.add_root(dst_node);
                }

                Compare { dst, lhs, cmp, rhs } => {
                    let lhs_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*lhs),
                        children: vec![],
                    });

                    let rhs_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*rhs),
                        children: vec![],
                    });

                    let cmp_kind = match cmp {
                        CompareOp::Lt => OpKind::Lt,
                        CompareOp::Le => OpKind::Le,
                        CompareOp::Eq => OpKind::Eq,
                        CompareOp::Ge => OpKind::Ge,
                        CompareOp::Gt => OpKind::Gt,
                    };

                    let cmp_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(cmp_kind),
                        children: vec![lhs_node, rhs_node],
                    });

                    let dst_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*dst)),
                        children: vec![cmp_node],
                    });

                    forest.add_root(dst_node);
                }

                Load { dst, src } => {
                    let src_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*src)),
                        children: vec![],
                    });

                    let load_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Load),
                        children: vec![src_node],
                    });

                    let dst_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*dst)),
                        children: vec![load_node],
                    });

                    forest.add_root(dst_node);
                }

                Store { dst, src } => {
                    let dst_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(Value::Variable(*dst)),
                        children: vec![],
                    });

                    let src_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*src),
                        children: vec![],
                    });

                    let store_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Store),
                        children: vec![dst_node, src_node],
                    });

                    forest.add_root(store_node);
                }

                Return => {
                    let ret_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Ret),
                        children: vec![],
                    });

                    forest.add_root(ret_node);
                }

                ReturnValue(val) => {
                    let val_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*val),
                        children: vec![],
                    });

                    let ret_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Ret),
                        children: vec![val_node],
                    });

                    forest.add_root(ret_node);
                }

                Branch(_) => {
                    let br_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Br),
                        children: vec![],
                    });

                    forest.add_root(br_node);
                }

                BranchCond { cond, .. } => {
                    let cond_node = forest.alloc(SFNode {
                        kind: NodeKind::Value(*cond),
                        children: vec![],
                    });

                    let br_node = forest.alloc(SFNode {
                        kind: NodeKind::Op(OpKind::Br),
                        children: vec![cond_node],
                    });

                    forest.add_root(br_node);
                }

                Label(_) | Call { .. } | CallResult { .. } => panic!("illegal context instruction"),
            }
        }

        forest
    }

    pub fn alloc(&mut self, node: SFNode) -> NodeId {
        let id = self.arena.len();
        self.arena.push(node);
        id
    }

    pub fn add_root(&mut self, id: NodeId) {
        self.roots.push(id);
    }
}

pub fn generate_forests(contexts: &[Context]) -> Vec<SelectionForest> {
    contexts.iter().map(SelectionForest::new).collect()
}
