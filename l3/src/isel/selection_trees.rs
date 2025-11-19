use l3::*;
use std::collections::BTreeSet;

use crate::isel::contexts::Context;

pub type NodeId = usize;

#[derive(Debug)]
pub enum OpKind {
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
pub struct SelectionNode {
    pub kind: NodeKind,
    pub children: Vec<NodeId>,
}

#[derive(Debug)]
pub struct SelectionForest {
    pub arena: Vec<SelectionNode>,
    pub roots: BTreeSet<NodeId>,
}

impl SelectionForest {
    pub fn new(context: &Context) -> Self {
        todo!()
    }
}
