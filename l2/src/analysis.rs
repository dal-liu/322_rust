use l2::*;
use std::collections::HashMap;

pub fn compute_targets(prog: &mut Program) {
    for func in &mut prog.functions {
        let blocks = &mut func.basic_blocks;
        let last_index = blocks.len().saturating_sub(1);

        let label_to_index: HashMap<_, _> = blocks
            .iter()
            .enumerate()
            .filter_map(|(i, block)| {
                if let Some(Instruction::Label(label)) = block.instructions.first() {
                    Some((label.clone(), i))
                } else {
                    None
                }
            })
            .collect();

        for i in 0..blocks.len() {
            let block = &mut blocks[i];
            let last_inst = block.instructions.last();

            block.target = match last_inst {
                Some(Instruction::CJump { label, .. }) => {
                    let j = label_to_index
                        .get(label)
                        .copied()
                        .unwrap_or_else(|| panic!("invalid label {}", label));
                    let k = if i < last_index { Some(i + 1) } else { None };
                    Target::Indexes([Some(j), k])
                }
                Some(Instruction::Goto(label)) => {
                    let j = label_to_index
                        .get(label)
                        .copied()
                        .unwrap_or_else(|| panic!("invalid label {}", label));
                    Target::Indexes([Some(j), None])
                }
                Some(Instruction::Return) => Target::Indexes([None, None]),
                Some(_) => {
                    let k = if i < last_index { Some(i + 1) } else { None };
                    Target::Indexes([k, None])
                }
                None => panic!("empty block in {}", &func.name),
            }
        }
    }
}
