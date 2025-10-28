mod interference;

use crate::analysis::AnalysisResult;
use crate::regalloc::interference::InterferenceGraph;

use l2::*;

pub fn compute_interference(func: &Function, liveness: &AnalysisResult) -> InterferenceGraph {
    InterferenceGraph::build(func, liveness)
}
