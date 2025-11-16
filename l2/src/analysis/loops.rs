use l2::*;
use std::collections::HashMap;

use crate::analysis::dominators::DominatorTree;
use crate::bitvector::BitVector;

pub type LoopId = usize;

#[derive(Debug)]
pub struct LoopForest {
    pub loops: Vec<Loop>,
    pub block_map: HashMap<BlockId, LoopId>,
}

impl LoopForest {
    pub fn new(func: &Function, dt: &DominatorTree) -> Self {
        let cfg = &func.cfg;
        let num_blocks = func.basic_blocks.len();

        let back_edges = func.basic_blocks.iter().flat_map(|block| {
            let latch = block.id.0;
            cfg.successors[latch].iter().filter_map(move |succ| {
                let header = succ.0;
                dt.dominates(header, latch).then_some((latch, header))
            })
        });

        let natural_loops = back_edges.map(|(latch, header)| {
            let mut stack = vec![latch];
            let mut loop_blocks = BitVector::new(num_blocks);
            loop_blocks.set(header);

            while let Some(node) = stack.pop() {
                if !loop_blocks.test(node) {
                    loop_blocks.set(node);
                    stack.extend(cfg.predecessors[node].iter().map(|id| id.0));
                }
            }

            (header, loop_blocks)
        });

        let mut merged_loops: Vec<Loop> = natural_loops
            .fold(
                vec![BitVector::new(num_blocks); num_blocks],
                |mut merged_loops, (header, blocks)| {
                    merged_loops[header].union(&blocks);
                    merged_loops
                },
            )
            .into_iter()
            .enumerate()
            .filter_map(|(header, blocks)| {
                blocks.any().then_some(Loop {
                    header: BlockId(header),
                    basic_blocks: blocks.iter().map(BlockId).collect(),
                    depth: 0,
                    children: Vec::new(),
                })
            })
            .collect();
        merged_loops.sort_by_key(|loop_| loop_.basic_blocks.len());

        let mut roots = Vec::new();
        let mut block_map = HashMap::new();

        for i in 0..merged_loops.len() {
            merged_loops
                .iter()
                .flat_map(|loop_| &loop_.basic_blocks)
                .for_each(|id| {
                    block_map.entry(id.clone()).or_insert(i);
                });

            let (first, second) = merged_loops.split_at_mut(i + 1);
            let loop_ = &first[i];

            let parent = second.iter_mut().find(|other| {
                dt.dominates(other.header.0, loop_.header.0)
                    && other.basic_blocks.contains(&loop_.header)
            });

            match parent {
                Some(parent) => parent.children.push(i),
                None => roots.push(i),
            }
        }

        let mut stack: Vec<(LoopId, u32)> = roots.iter().map(|&root| (root, 1)).collect();
        while let Some((node, depth)) = stack.pop() {
            let loop_ = &mut merged_loops[node];
            loop_.depth = depth;
            for &child in &loop_.children {
                stack.push((child, depth + 1));
            }
        }

        Self {
            loops: merged_loops,
            block_map,
        }
    }

    pub fn loop_depth(&self, block: &BlockId) -> u32 {
        match self.block_map.get(block) {
            Some(&loop_id) => self.loops[loop_id].depth,
            None => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Loop {
    pub header: BlockId,
    pub basic_blocks: Vec<BlockId>,
    pub depth: u32,
    pub children: Vec<LoopId>,
}

pub fn compute_loops(func: &Function, dt: &DominatorTree) -> LoopForest {
    LoopForest::new(func, dt)
}
