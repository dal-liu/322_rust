use l2::*;

use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

#[derive(Debug)]
pub struct LivenessResult {
    pub interner: Interner<Value>,
    pub out: Vec<BitVector>,
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
        let node = id.0;

        out[node].clear();
        for succ in &cfg.successors[node] {
            out[node].union(&in_[succ.0]);
        }

        let mut temp = out[node].clone();
        temp.difference(&kill[node]);
        temp.union(&gen_[node]);

        if temp != in_[node] {
            in_[node] = temp;
            worklist.extend(cfg.predecessors[node].iter());
        }
    }

    LivenessResult { interner, out }
}

fn value_interner(func: &Function) -> Interner<Value> {
    let mut value_interner = Interner::new();

    for &reg in Register::GP_REGISTERS {
        value_interner.intern(Value::Register(reg));
    }

    func.basic_blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .flat_map(|inst| inst.uses().into_iter().chain(inst.defs()))
        .for_each(|var| {
            value_interner.intern(var);
        });

    value_interner
}
