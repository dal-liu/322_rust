use crate::regalloc::interference::InterferenceGraph;

use l2::*;

fn simplify(graph: &mut InterferenceGraph) -> Vec<usize> {
    let mut stack = Vec::new();

    while !graph.is_empty() {
        let node = (0..graph.len())
            .filter(|&n| graph.degree(n) > 0 && graph.degree(n) < NUM_GP_REGISTERS)
            .max_by_key(|&n| graph.degree(n))
            .or_else(|| (0..graph.len()).max_by_key(|&n| graph.degree(n)))
            .expect("interference graph should not be empty");

        debug_assert!(graph.degree(node) > 0);

        graph.remove_node(node);
        stack.push(node);
    }

    stack
}

pub fn color_graph(graph: &mut InterferenceGraph) {
    simplify(graph);
}
