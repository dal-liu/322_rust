mod interference;
mod spilling;

use crate::analysis::LivenessResult;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;

pub fn compute_interference(func: &Function, liveness: &LivenessResult) -> InterferenceGraph {
    InterferenceGraph::build(func, liveness)
}
