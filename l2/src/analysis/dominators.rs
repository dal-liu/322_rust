use l2::*;

use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

#[derive(Debug)]
pub struct DominatorTree {
    pub preorder: Vec<u32>,
    pub postorder: Vec<u32>,
}

impl DominatorTree {
    pub fn new(func: &Function) -> Self {
        let num_blocks = func.basic_blocks.len();
        let cfg = &func.cfg;
        let entry_id = &func.basic_blocks[0].id;

        let mut sdom = vec![BitVector::new(num_blocks); num_blocks];
        for i in 0..num_blocks {
            sdom[i].set_from(0..num_blocks);
        }

        let mut worklist = Worklist::new();
        worklist.push(&entry_id);

        while let Some(id) = worklist.pop() {
            let node = id.0;
            let mut temp = BitVector::new(num_blocks);

            if node != entry_id.0 {
                temp.set_from(0..num_blocks);
                for pred in &cfg.predecessors[node] {
                    temp.intersection(&sdom[pred.0]);
                }
            }

            temp.set(node);

            if temp != sdom[node] {
                sdom[node] = temp;
                worklist.extend(cfg.successors[node].iter());
            }
        }

        for i in 0..num_blocks {
            sdom[i].reset(i);
        }

        let idom: Vec<Option<usize>> = sdom
            .iter()
            .map(|dom| dom.iter().max_by_key(|&n| sdom[n].count()))
            .collect();

        let mut tree = vec![Vec::new(); num_blocks];
        for (node, &parent) in idom.iter().enumerate() {
            if let Some(parent) = parent {
                tree[parent].push(node);
            }
        }

        enum TraversalState {
            Entering,
            Exiting,
        }

        let mut counter = 0;
        let mut preorder = vec![0; num_blocks];
        let mut postorder = vec![0; num_blocks];
        let mut stack = vec![(entry_id.0, TraversalState::Entering)];

        while let Some((node, state)) = stack.pop() {
            match state {
                TraversalState::Entering => {
                    preorder[node] = counter;

                    stack.push((node, TraversalState::Exiting));

                    for &child in tree[node].iter().rev() {
                        stack.push((child, TraversalState::Entering));
                    }
                }
                TraversalState::Exiting => {
                    postorder[node] = counter;
                }
            }

            counter += 1;
        }

        Self {
            preorder,
            postorder,
        }
    }

    pub fn dominates(&self, u: &BlockId, v: &BlockId) -> bool {
        self.preorder[u.0] <= self.preorder[v.0] && self.postorder[u.0] >= self.postorder[v.0]
    }
}

pub fn compute_dominators(func: &Function) -> DominatorTree {
    DominatorTree::new(func)
}
