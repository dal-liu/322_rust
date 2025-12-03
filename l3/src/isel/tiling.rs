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

#[derive(Debug)]
struct Pattern {
    children: Vec<Pattern>,
    matches: fn(&SFNode, Option<Value>) -> bool,
}

#[derive(Debug)]
struct Tile {
    pattern: Pattern,
    cost: u32,
}

impl Tile {
    fn new(pattern: Pattern, cost: u32) -> Self {
        Self { pattern, cost }
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
struct TilingSelector {
    tiles: Vec<Tile>,
}

impl TilingSelector {
    pub fn new() -> Self {
        let assign = Tile::new(pat!(Assign(pat!(any)) -> any), 1);
        let load = Tile::new(pat!(Load(pat!(any)) -> any), 1);
        let store = Tile::new(pat!(Store(pat!(any), pat!(any)) -> any), 1);

        let mut tiles = vec![assign, load, store];
        tiles.sort_by_key(|tile| (Reverse(tile.size()), tile.cost));

        Self { tiles }
    }
}
