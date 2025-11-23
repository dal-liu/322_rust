use common::{BitVector, Worklist};
use l3::*;

pub trait Dataflow {
    const DIRECTION: Direction;

    fn boundary(&self) -> BitVector;

    fn meet(&self, current: &mut BitVector, other: &BitVector);

    fn transfer(&self, input: &BitVector, id: BlockId) -> BitVector;
}

#[derive(Debug)]
pub enum Direction {
    Forward,
    Backward,
}

pub fn solve<T: Dataflow>(func: &Function, dataflow: &T) -> (Vec<BitVector>, Vec<BitVector>) {
    let num_blocks = func.basic_blocks.len();
    let mut block_enter = vec![dataflow.boundary(); num_blocks];
    let mut block_exit = vec![dataflow.boundary(); num_blocks];
    let mut worklist = Worklist::new();

    match T::DIRECTION {
        Direction::Forward => worklist.extend((0..num_blocks).rev().map(BlockId)),
        Direction::Backward => worklist.extend((0..num_blocks).map(BlockId)),
    };

    while let Some(id) = worklist.pop() {
        let i = id.0;
        let cfg = &func.cfg;

        block_enter[i] = dataflow.boundary();

        let enter_neighbors = match T::DIRECTION {
            Direction::Forward => &cfg.predecessors[i],
            Direction::Backward => &cfg.successors[i],
        };
        for neighbor in enter_neighbors {
            dataflow.meet(&mut block_enter[i], &block_exit[neighbor.0]);
        }

        let temp = dataflow.transfer(&block_enter[i], id);

        if temp != block_exit[i] {
            block_exit[i] = temp;

            let exit_neighbors = match T::DIRECTION {
                Direction::Forward => &cfg.successors[i],
                Direction::Backward => &cfg.predecessors[i],
            };
            worklist.extend(exit_neighbors.iter().copied());
        }
    }

    (block_enter, block_exit)
}
