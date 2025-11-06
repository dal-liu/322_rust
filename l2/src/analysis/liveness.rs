use crate::analysis::worklist::Worklist;
use crate::bitvector::BitVector;

use l2::*;

#[derive(Debug)]
#[allow(dead_code)]
pub struct LivenessResult {
    pub interner: Interner<Value>,
    pub gen_: Vec<Vec<BitVector>>,
    pub kill: Vec<Vec<BitVector>>,
    pub in_: Vec<Vec<BitVector>>,
    pub out: Vec<Vec<BitVector>>,
}

impl DisplayResolved for LivenessResult {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter,
        interner: &Interner<String>,
    ) -> std::fmt::Result {
        writeln!(f, "(\n(in")?;

        for vec in &self.in_ {
            for bit in vec {
                let mut line: Vec<String> = bit
                    .iter()
                    .map(|val| self.interner.resolve(val).resolved(interner).to_string())
                    .collect();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n(out")?;

        for vec in &self.out {
            for bit in vec {
                let mut line: Vec<String> = bit
                    .iter()
                    .map(|val| self.interner.resolve(val).resolved(interner).to_string())
                    .collect();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n)")
    }
}

fn empty_dataflow_set(func: &Function, len: usize) -> Vec<Vec<BitVector>> {
    func.basic_blocks
        .iter()
        .map(|block| vec![BitVector::new(len); block.instructions.len()])
        .collect()
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
    let mut block_gen: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut block_kill: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for inst in &block.instructions {
            block_gen[i].set_from(inst.uses().into_iter().filter_map(|use_| {
                let index = interner.intern(use_);
                (!block_kill[i].test(index)).then_some(index)
            }));
            block_kill[i].set_from(inst.defs().into_iter().map(|def| interner.intern(def)));
        }
    }

    let cfg = &func.cfg;
    let mut block_in: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut block_out: Vec<BitVector> = vec![BitVector::new(num_gp_variables); num_blocks];
    let mut worklist = Worklist::new();
    worklist.extend(func.basic_blocks.iter().map(|block| &block.id));

    while let Some(id) = worklist.pop() {
        let i = id.0;

        block_out[i].clear();
        for succ in &cfg.successors[i] {
            block_out[i].union(&block_in[succ.0]);
        }

        let mut temp = block_out[i].clone();
        temp.difference(&block_kill[i]);
        temp.union(&block_gen[i]);

        if temp != block_in[i] {
            block_in[i] = temp;
            worklist.extend(cfg.predecessors[i].iter());
        }
    }

    let mut gen_ = empty_dataflow_set(func, num_gp_variables);
    let mut kill = empty_dataflow_set(func, num_gp_variables);
    let mut in_ = empty_dataflow_set(func, num_gp_variables);
    let mut out = empty_dataflow_set(func, num_gp_variables);

    for block in &func.basic_blocks {
        let i = block.id.0;

        for (j, inst) in block.instructions.iter().enumerate().rev() {
            gen_[i][j].set_from(inst.uses().into_iter().map(|def| interner.intern(def)));
            kill[i][j].set_from(inst.defs().into_iter().map(|def| interner.intern(def)));

            out[i][j] = if j == block.instructions.len() - 1 {
                block_out[i].clone()
            } else {
                in_[i][j + 1].clone()
            };

            in_[i][j] = out[i][j].clone();
            in_[i][j].difference(&kill[i][j]);
            in_[i][j].union(&gen_[i][j]);
        }
    }

    LivenessResult {
        interner,
        gen_,
        kill,
        in_,
        out,
    }
}
