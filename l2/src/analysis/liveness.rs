use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

use l2::*;

#[derive(Debug)]
#[allow(dead_code)]
pub struct LivenessResult {
    pub interner: Interner<Value>,
    pub in_: Vec<BitVector>,
    pub out: Vec<BitVector>,
}

fn value_interner(func: &Function) -> Interner<Value> {
    let mut interner = Interner::new();

    for &reg in Register::GP_REGISTERS {
        interner.intern(Value::Register(reg));
    }

    func.basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .flat_map(|inst| inst.uses().into_iter().chain(inst.defs()))
        .for_each(|var| {
            interner.intern(var);
        });

    interner
}

pub fn compute_liveness(func: &Function) -> LivenessResult {
    let mut interner = value_interner(func);
    let num_gp_variables = interner.len();
    let num_blocks = func.basic_blocks.len();
    let mut gen_: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut kill: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for inst in &block.instructions {
            gen_[i].set_from(inst.uses().into_iter().filter_map(|use_| {
                let index = interner.intern(use_);
                (!kill[i].test(index)).then_some(index)
            }));
            kill[i].set_from(inst.defs().into_iter().map(|def| interner.intern(def)));
        }
    }

    let cfg = &func.cfg;
    let mut in_: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut out: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut worklist = Worklist::new();
    worklist.extend(func.basic_blocks.iter().map(|block| &block.id));

    while let Some(id) = worklist.pop() {
        let i = id.0;

        out[i].clear();
        for succ in &cfg.successors[i] {
            out[i].union(&in_[succ.0]);
        }

        let mut temp = out[i].clone();
        temp.difference(&kill[i]);
        temp.union(&gen_[i]);

        if temp != in_[i] {
            in_[i] = temp;
            worklist.extend(cfg.predecessors[i].iter());
        }
    }

    LivenessResult { interner, in_, out }
}
