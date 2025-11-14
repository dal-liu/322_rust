use l2::*;

pub fn remove_redundant_moves(func: &mut Function) {
    for block in &mut func.basic_blocks {
        block.instructions.retain(|inst| match inst {
            Instruction::Assign { dst, src } => dst != src,
            _ => true,
        });
    }
}
