use l2::*;

use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

#[derive(Debug)]
pub struct DominatorTree {
    pub preorder: Vec<usize>,
    pub postorder: Vec<usize>,
}

impl DominatorTree {
    pub fn new(func: &Function) -> Self {
        let num_blocks = func.basic_blocks.len();
        let mut sdom = vec![BitVector::new(num_blocks); num_blocks];
        for i in 0..num_blocks {
            sdom[i].set_from(0..num_blocks);
        }

        let cfg = &func.cfg;
        let entry_id = &func.basic_blocks[0].id;
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

        // TODO: find a way to do this iteratively
        fn dfs(
            node: usize,
            tree: &Vec<Vec<usize>>,
            counter: &mut usize,
            preorder: &mut Vec<usize>,
            postorder: &mut Vec<usize>,
        ) {
            preorder[node] = *counter;
            *counter += 1;

            for &child in &tree[node] {
                dfs(child, tree, counter, preorder, postorder);
            }

            postorder[node] = *counter;
            *counter += 1;
        }

        let mut counter = 0;
        let mut preorder = vec![0; num_blocks];
        let mut postorder = vec![0; num_blocks];

        dfs(
            entry_id.0,
            &mut tree,
            &mut counter,
            &mut preorder,
            &mut postorder,
        );

        Self {
            preorder,
            postorder,
        }
    }

    pub fn dominates(&self, u: usize, v: usize) -> bool {
        self.preorder[u] <= self.preorder[v] && self.postorder[u] >= self.postorder[v]
    }
}

pub fn compute_dominators(func: &Function) -> DominatorTree {
    DominatorTree::new(func)
}
