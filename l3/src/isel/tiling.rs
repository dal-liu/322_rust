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

    ($kind:ident) => {
        Pattern {
            children: Vec::new(),
            matches: |node, _| matches!(&node.kind, NodeKind::Op { kind: OpKind::$kind, result: None }),
        }
    };

    ($kind:ident($($child:expr),*)) => {
        Pattern {
            children: vec![$($child),*],
            matches: |node, _| matches!(&node.kind, NodeKind::Op { kind: OpKind::$kind, result: None }),
        }
    };

    ($kind:ident($($child:expr),*) -> any) => {
        Pattern {
            children: vec![$($child),*],
            matches: |node, _| matches!(&node.kind, NodeKind::Op { kind: OpKind::$kind, result: Some(_) }),
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
    cost: u32,
    emit: fn(&SelectionForest, NodeId) -> Vec<l2::Instruction>,
}

impl Tile {
    fn new(
        pattern: Pattern,
        cost: u32,
        emit: fn(&SelectionForest, NodeId) -> Vec<l2::Instruction>,
    ) -> Self {
        Self {
            pattern,
            cost,
            emit,
        }
    }

    fn size(&self) -> u32 {
        fn dfs(pat: &Pattern) -> u32 {
            1 + pat.children.iter().map(|child| dfs(child)).sum::<u32>()
        }
        dfs(&self.pattern)
    }

    fn matches(&self, forest: &SelectionForest, root: NodeId) -> bool {
        fn dfs(forest: &SelectionForest, id: NodeId, opt: Option<Value>, pat: &Pattern) -> bool {
            let node = &forest.arena[id];
            if !(pat.matches)(node, opt) {
                false
            } else if pat.children.is_empty() {
                true
            } else {
                pat.children.len() == node.children.len()
                    && node
                        .children
                        .iter()
                        .zip(&pat.children)
                        .all(|(&i, p)| dfs(forest, i, opt, p))
            }
        }

        let NodeKind::Op { result: opt, .. } = forest.arena[root].kind else {
            unreachable!("roots should be ops");
        };

        dfs(forest, root, opt, &self.pattern)
    }
}

#[derive(Debug)]
pub struct TilingSelector {
    pub tiles: Vec<Tile>,
}

impl TilingSelector {
    pub fn new() -> Self {
        use l2::Instruction as L2;

        let assign = Tile::new(pat!(Assign(pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Assign {
                dst: translate_node(forest, root),
                src: translate_node(forest, forest.arena[root].children[0]),
            }]
        });

        let add_assign = Tile::new(pat!(Add(pat!(any), pat!(any)) -> any), 2, |forest, root| {
            let dst = translate_node(forest, root);
            vec![
                L2::Assign {
                    dst,
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Arithmetic {
                    dst,
                    aop: l2::ArithmeticOp::AddAssign,
                    src: translate_node(forest, forest.arena[root].children[1]),
                },
            ]
        });

        let sub_assign = Tile::new(pat!(Sub(pat!(any), pat!(any)) -> any), 2, |forest, root| {
            let dst = translate_node(forest, root);
            vec![
                L2::Assign {
                    dst,
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Arithmetic {
                    dst,
                    aop: l2::ArithmeticOp::SubAssign,
                    src: translate_node(forest, forest.arena[root].children[1]),
                },
            ]
        });

        let mul_assign = Tile::new(pat!(Mul(pat!(any), pat!(any)) -> any), 2, |forest, root| {
            let dst = translate_node(forest, root);
            vec![
                L2::Assign {
                    dst,
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Arithmetic {
                    dst,
                    aop: l2::ArithmeticOp::MulAssign,
                    src: translate_node(forest, forest.arena[root].children[1]),
                },
            ]
        });

        let bit_and_assign = Tile::new(
            pat!(BitAnd(pat!(any), pat!(any)) -> any),
            2,
            |forest, root| {
                let dst = translate_node(forest, root);
                vec![
                    L2::Assign {
                        dst,
                        src: translate_node(forest, forest.arena[root].children[0]),
                    },
                    L2::Arithmetic {
                        dst,
                        aop: l2::ArithmeticOp::BitAndAssign,
                        src: translate_node(forest, forest.arena[root].children[1]),
                    },
                ]
            },
        );

        let shl_assign = Tile::new(pat!(Shl(pat!(any), pat!(any)) -> any), 2, |forest, root| {
            let dst = translate_node(forest, root);
            vec![
                L2::Assign {
                    dst,
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Shift {
                    dst,
                    sop: l2::ShiftOp::ShlAssign,
                    src: translate_node(forest, forest.arena[root].children[1]),
                },
            ]
        });

        let shr_assign = Tile::new(pat!(Shr(pat!(any), pat!(any)) -> any), 2, |forest, root| {
            let dst = translate_node(forest, root);
            vec![
                L2::Assign {
                    dst,
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Shift {
                    dst,
                    sop: l2::ShiftOp::ShrAssign,
                    src: translate_node(forest, forest.arena[root].children[1]),
                },
            ]
        });

        let lt = Tile::new(pat!(Lt(pat!(any), pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Compare {
                dst: translate_node(forest, root),
                lhs: translate_node(forest, forest.arena[root].children[0]),
                cmp: l2::CompareOp::Lt,
                rhs: translate_node(forest, forest.arena[root].children[1]),
            }]
        });

        let le = Tile::new(pat!(Le(pat!(any), pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Compare {
                dst: translate_node(forest, root),
                lhs: translate_node(forest, forest.arena[root].children[0]),
                cmp: l2::CompareOp::Le,
                rhs: translate_node(forest, forest.arena[root].children[1]),
            }]
        });

        let eq = Tile::new(pat!(Eq(pat!(any), pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Compare {
                dst: translate_node(forest, root),
                lhs: translate_node(forest, forest.arena[root].children[0]),
                cmp: l2::CompareOp::Eq,
                rhs: translate_node(forest, forest.arena[root].children[1]),
            }]
        });

        let ge = Tile::new(pat!(Ge(pat!(any), pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Compare {
                dst: translate_node(forest, root),
                lhs: translate_node(forest, forest.arena[root].children[1]),
                cmp: l2::CompareOp::Le,
                rhs: translate_node(forest, forest.arena[root].children[0]),
            }]
        });

        let gt = Tile::new(pat!(Gt(pat!(any), pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Compare {
                dst: translate_node(forest, root),
                lhs: translate_node(forest, forest.arena[root].children[1]),
                cmp: l2::CompareOp::Lt,
                rhs: translate_node(forest, forest.arena[root].children[0]),
            }]
        });

        let load = Tile::new(pat!(Load(pat!(any)) -> any), 1, |forest, root| {
            vec![L2::Load {
                dst: translate_node(forest, root),
                src: translate_node(forest, forest.arena[root].children[0]),
                offset: 0,
            }]
        });

        let store = Tile::new(pat!(Store(pat!(any), pat!(any))), 1, |forest, root| {
            vec![L2::Store {
                dst: translate_node(forest, forest.arena[root].children[0]),
                offset: 0,
                src: translate_node(forest, forest.arena[root].children[1]),
            }]
        });

        let return_ = Tile::new(pat!(Return), 1, |_, _| vec![L2::Return]);

        let return_value = Tile::new(pat!(Return(pat!(any))), 2, |forest, root| {
            vec![
                L2::Assign {
                    dst: l2::Value::Register(l2::Register::RAX),
                    src: translate_node(forest, forest.arena[root].children[0]),
                },
                L2::Return,
            ]
        });

        let branch = Tile::new(pat!(Branch(pat!(any))), 1, |forest, root| {
            let NodeKind::Value(Value::Label(label)) =
                forest.arena[forest.arena[root].children[0]].kind
            else {
                unreachable!("branch node should have label");
            };
            vec![L2::Goto(l2::SymbolId(label.0))]
        });

        let branch_cond = Tile::new(pat!(Branch(pat!(any), pat!(any))), 1, |forest, root| {
            let NodeKind::Value(Value::Label(label)) =
                forest.arena[forest.arena[root].children[1]].kind
            else {
                unreachable!("branch node should have label");
            };
            vec![L2::CJump {
                lhs: translate_node(forest, forest.arena[root].children[1]),
                cmp: l2::CompareOp::Eq,
                rhs: l2::Value::Number(1),
                label: l2::SymbolId(label.0),
            }]
        });

        let mut tiles = vec![
            assign,
            add_assign,
            sub_assign,
            mul_assign,
            bit_and_assign,
            shl_assign,
            shr_assign,
            le,
            lt,
            eq,
            ge,
            gt,
            load,
            store,
            return_,
            return_value,
            branch,
            branch_cond,
        ];
        tiles.sort_by_key(|tile| (Reverse(tile.size()), tile.cost));

        Self { tiles }
    }
}

fn translate_node(forest: &SelectionForest, id: NodeId) -> l2::Value {
    match &forest.arena[id].kind {
        NodeKind::Op { result, .. } => {
            let Some(Value::Variable(res)) = result else {
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
