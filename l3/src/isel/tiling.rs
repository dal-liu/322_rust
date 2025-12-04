use std::cmp::Reverse;

use l2;
use l3::*;

use crate::isel::forest::{NodeKind, OpKind, SFNode, SelectionForest};

macro_rules! pat {
    (any) => {
        Pattern {
            children: Vec::new(),
            matches: |node, _| {
                matches!(
                    &node.kind,
                    NodeKind::Op {
                        result: Some(_),
                        ..
                    } | NodeKind::Value(_)
                )
            },
        }
    };

    (exact) => {
        Pattern {
            children: Vec::new(),
            matches: |node, opt| match &node.kind {
                NodeKind::Op {
                    result: Some(res), ..
                } => Some(Value::Variable(*res)) == opt,
                NodeKind::Value(val) => Some(*val) == opt,
                _ => false,
            },
        }
    };

    ($kind:ident($($child:expr),*) -> any) => {
        Pattern {
            children: vec![$($child),*],
            matches: |node, _| matches!(&node.kind, NodeKind::Op { kind: OpKind::$kind, .. }),
        }
    };

    ($kind:ident($($child:expr),*) -> exact) => {
        Pattern {
            children: vec![$($child),*],
            matches: |node, opt| {
                matches!(
                    &node.kind,
                    NodeKind::Op {
                        kind: OpKind::$kind,
                        result: Some(res)
                    } if Some(Value::Variable(*res)) == opt
                )
            },
        }
    };
}

type NodeId = usize;
type TileId = usize;

#[derive(Debug)]
struct Pattern {
    children: Vec<Pattern>,
    matches: fn(&SFNode, Option<Value>) -> bool,
}

#[derive(Debug)]
pub struct Tile {
    pattern: Pattern,
    ordered: bool,
    cost: u32,
    emit: fn(&SelectionForest, NodeId) -> Vec<l2::Instruction>,
}

impl Tile {
    fn new(
        pattern: Pattern,
        ordered: bool,
        emit: fn(&SelectionForest, NodeId) -> Vec<l2::Instruction>,
        cost: u32,
    ) -> Self {
        Self {
            pattern,
            ordered,
            emit,
            cost,
        }
    }

    fn size(&self) -> u32 {
        let mut size = 0;
        let mut stack = vec![&self.pattern];
        while let Some(pat) = stack.pop() {
            size += 1;
            stack.extend(pat.children.iter());
        }
        size
    }
}

#[derive(Debug)]
pub struct TilingSelector {
    pub tiles: Vec<Tile>,
}

impl TilingSelector {
    pub fn new() -> Self {
        let assign = Tile::new(
            pat!(Assign(pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let src = node_value(&forest.arena[forest.arena[root].children[0]]);
                vec![l2::Instruction::Assign { dst, src }]
            },
            1,
        );

        let load = Tile::new(
            pat!(Load(pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let src = node_value(&forest.arena[forest.arena[root].children[0]]);
                vec![l2::Instruction::Load {
                    dst,
                    src,
                    offset: 0,
                }]
            },
            1,
        );

        let store = Tile::new(
            pat!(Store(pat!(any), pat!(any)) -> any),
            true,
            |forest, root| {
                let dst = node_value(&forest.arena[forest.arena[root].children[0]]);
                let src = node_value(&forest.arena[forest.arena[root].children[1]]);
                vec![l2::Instruction::Store {
                    dst,
                    offset: 0,
                    src,
                }]
            },
            1,
        );

        let add = Tile::new(
            pat!(Add(pat!(any), pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let lhs = node_value(&forest.arena[forest.arena[root].children[0]]);
                let rhs = node_value(&forest.arena[forest.arena[root].children[1]]);
                vec![
                    l2::Instruction::Assign { dst, src: lhs },
                    l2::Instruction::Arithmetic {
                        dst,
                        aop: l2::ArithmeticOp::AddAssign,
                        src: rhs,
                    },
                ]
            },
            2,
        );

        let sub = Tile::new(
            pat!(Sub(pat!(any), pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let lhs = node_value(&forest.arena[forest.arena[root].children[0]]);
                let rhs = node_value(&forest.arena[forest.arena[root].children[1]]);
                vec![
                    l2::Instruction::Assign { dst, src: lhs },
                    l2::Instruction::Arithmetic {
                        dst,
                        aop: l2::ArithmeticOp::SubAssign,
                        src: rhs,
                    },
                ]
            },
            2,
        );

        let mul = Tile::new(
            pat!(Mul(pat!(any), pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let lhs = node_value(&forest.arena[forest.arena[root].children[0]]);
                let rhs = node_value(&forest.arena[forest.arena[root].children[1]]);
                vec![
                    l2::Instruction::Assign { dst, src: lhs },
                    l2::Instruction::Arithmetic {
                        dst,
                        aop: l2::ArithmeticOp::MulAssign,
                        src: rhs,
                    },
                ]
            },
            2,
        );

        let bit_and = Tile::new(
            pat!(BitAnd(pat!(any), pat!(any)) -> any),
            false,
            |forest, root| {
                let dst = node_value(&forest.arena[root]);
                let lhs = node_value(&forest.arena[forest.arena[root].children[0]]);
                let rhs = node_value(&forest.arena[forest.arena[root].children[1]]);
                vec![
                    l2::Instruction::Assign { dst, src: lhs },
                    l2::Instruction::Arithmetic {
                        dst,
                        aop: l2::ArithmeticOp::BitAndAssign,
                        src: rhs,
                    },
                ]
            },
            2,
        );

        let mut tiles = vec![assign, load, store, add, sub, mul, bit_and];
        tiles.sort_by_key(|tile| (Reverse(tile.size()), tile.cost));

        Self { tiles }
    }
}

fn node_value(node: &SFNode) -> l2::Value {
    match &node.kind {
        NodeKind::Op { result, .. } => {
            let Some(res) = result else {
                unreachable!("op should have a result");
            };
            l2::Value::Variable(l2::SymbolId(res.0))
        }
        NodeKind::Value(val) => match val {
            Value::Number(num) => l2::Value::Number(*num),
            Value::Label(label) => l2::Value::Label(l2::SymbolId(label.0)),
            Value::Function(callee) => l2::Value::Function(l2::SymbolId(callee.0)),
            Value::Variable(var) => l2::Value::Function(l2::SymbolId(var.0)),
        },
    }
}
