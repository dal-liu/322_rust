mod peephole;

use crate::optimization::peephole::remove_redundant_moves;

use l2::*;

pub fn run_peephole_passes(func: &mut Function) {
    remove_redundant_moves(func);
}
