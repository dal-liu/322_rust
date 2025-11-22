use common::{BitVector, Worklist};
use l3::*;

pub trait Dataflow {
    const DIRECTION: Direction;

    fn boundary_condition(&self) -> BitVector;

    fn meet(&self, current: &mut BitVector, other: &BitVector);

    fn transfer(&self, input: &BitVector, id: BlockId) -> BitVector;
}

#[derive(Debug)]
pub enum Direction {
    Forward,
    Backward,
}

pub fn solve<T: Dataflow>(func: &Function, dataflow: &T) -> Vec<BitVector> {
    let num_blocks = func.basic_blocks.len();
    let mut block_sets = vec![dataflow.boundary_condition(); num_blocks];
    let mut worklist = Worklist::new();

    match T::DIRECTION {
        Direction::Forward => worklist.extend((0..num_blocks).rev().map(BlockId)),
        Direction::Backward => worklist.extend((0..num_blocks).map(BlockId)),
    };

    while let Some(id) = worklist.pop() {
        let i = id.0;
        let cfg = &func.cfg;
        let mut new_input_set = dataflow.boundary_condition();

        let neighbors_backward = match T::DIRECTION {
            Direction::Forward => &cfg.predecessors[i],
            Direction::Backward => &cfg.successors[i],
        };

        for neighbor in neighbors_backward {
            dataflow.meet(&mut new_input_set, &block_sets[neighbor.0]);
        }

        let new_output_set = dataflow.transfer(&new_input_set, id);

        if new_output_set != new_input_set {
            block_sets[i] = new_output_set;

            let neighbors_forward = match T::DIRECTION {
                Direction::Forward => &cfg.successors[i],
                Direction::Backward => &cfg.predecessors[i],
            };
            worklist.extend(neighbors_forward.iter().copied());
        }
    }

    block_sets
}
