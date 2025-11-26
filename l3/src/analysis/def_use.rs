use l3::*;
use utils::Interner;

use crate::analysis::ReachingDefResult;

#[derive(Debug)]
pub struct DefUseChain<'a> {
    interner: &'a Interner<Instruction>,
    users: Vec<Vec<&'a Instruction>>,
}

impl<'a> DefUseChain<'a> {
    pub fn new(func: &'a Function, reaching_def: &'a ReachingDefResult) -> Self {
        let interner = &reaching_def.interner;
        let num_insts = interner.len();
        let mut users = vec![Vec::new(); num_insts];

        for (i, block) in func.basic_blocks.iter().enumerate() {
            for (j, inst) in block.instructions.iter().enumerate() {
                for use_ in inst.uses() {
                    for def_id in &reaching_def.in_[i][j] {
                        if interner.resolve(def_id).defs() == Some(use_) {
                            users[def_id].push(inst);
                        }
                    }
                }
            }
        }

        Self { interner, users }
    }

    pub fn users_of(&self, inst: &Instruction) -> Vec<&Instruction> {
        self.users[self.interner[inst]].iter().copied().collect()
    }
}

pub fn build_def_use<'a>(
    func: &'a Function,
    reaching_def: &'a ReachingDefResult,
) -> DefUseChain<'a> {
    DefUseChain::new(func, reaching_def)
}
