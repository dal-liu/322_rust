use l2::*;
use std::collections::{HashSet, VecDeque};

#[derive(Debug)]
pub struct AnalysisResult {
    pub gen_: Vec<Vec<HashSet<Value>>>,
    pub kill: Vec<Vec<HashSet<Value>>>,
    pub in_: Vec<Vec<HashSet<Value>>>,
    pub out: Vec<Vec<HashSet<Value>>>,
}

impl DisplayResolved for AnalysisResult {
    fn fmt_with(&self, f: &mut std::fmt::Formatter, interner: &Interner) -> std::fmt::Result {
        writeln!(f, "(\n(in")?;

        for vec in &self.in_ {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n(out")?;

        for vec in &self.out {
            for set in vec {
                let mut line = set
                    .iter()
                    .map(|val| format!("{}", val.resolved(interner)))
                    .collect::<Vec<_>>();
                line.sort();
                writeln!(f, "({})", line.join(" "))?;
            }
        }

        writeln!(f, ")\n\n)")
    }
}

#[derive(Debug, Default)]
struct Worklist<'a> {
    queue: VecDeque<&'a BlockId>,
    set: HashSet<&'a BlockId>,
}

impl<'a> Worklist<'a> {
    pub fn extend<I: IntoIterator<Item = &'a BlockId>>(&mut self, indexes: I) {
        for i in indexes {
            if self.set.insert(i) {
                self.queue.push_back(i);
            }
        }
    }

    pub fn pop(&mut self) -> Option<&'a BlockId> {
        if let Some(index) = self.queue.pop_front() {
            self.set.remove(&index);
            Some(index)
        } else {
            None
        }
    }
}

pub fn compute_liveness(func: &Function) -> AnalysisResult {
    let num_blocks = func.basic_blocks.len();
    let mut block_gen: Vec<HashSet<Value>> = vec![HashSet::new(); num_blocks];
    let mut block_kill: Vec<HashSet<Value>> = vec![HashSet::new(); num_blocks];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        for inst in &block.instructions {
            block_gen[i].extend(
                uses(inst)
                    .into_iter()
                    .filter(|use_| !block_kill[i].contains(use_)),
            );
            block_kill[i].extend(defs(inst).into_iter());
        }
    }

    let cfg = &func.cfg;
    let mut block_in: Vec<HashSet<&Value>> = vec![HashSet::new(); num_blocks];
    let mut block_out: Vec<HashSet<&Value>> = vec![HashSet::new(); num_blocks];
    let mut worklist = Worklist::default();
    worklist.extend(func.basic_blocks.iter().map(|block| &block.id));

    while let Some(id) = worklist.pop() {
        let i = id.0;

        block_out[i] = cfg.successors[i]
            .iter()
            .flat_map(|succ| block_in[succ.0].iter().copied())
            .collect();

        let temp: HashSet<&Value> = block_gen[i]
            .iter()
            .chain(
                block_out[id.0]
                    .difference(&block_kill[i].iter().collect())
                    .copied(),
            )
            .collect();

        if temp != block_in[i] {
            block_in[i] = temp;
            worklist.extend(cfg.predecessors[i].iter());
        }
    }

    let mut gen_ = empty_dataflow_set(func);
    let mut kill = empty_dataflow_set(func);
    let mut in_ = empty_dataflow_set(func);
    let mut out = empty_dataflow_set(func);

    for block in &func.basic_blocks {
        let i = block.id.0;

        for (j, inst) in block.instructions.iter().enumerate().rev() {
            gen_[i][j].extend(uses(inst).into_iter());
            kill[i][j].extend(defs(inst).into_iter());

            out[i][j] = if j == block.instructions.len() - 1 {
                block_out[i].iter().map(|&val| val.clone()).collect()
            } else {
                in_[i][j + 1].clone()
            };

            in_[i][j] = gen_[i][j]
                .union(&out[i][j].difference(&kill[i][j]).cloned().collect())
                .cloned()
                .collect();
        }
    }

    AnalysisResult {
        gen_,
        kill,
        in_,
        out,
    }
}

fn uses(inst: &Instruction) -> Vec<Value> {
    use Instruction::*;
    use Register::*;

    match inst {
        Assign { src, .. } | Load { src, .. } => is_gp_variable(src)
            .then(|| vec![src.clone()])
            .unwrap_or_default(),

        Store { dst, src, .. }
        | Arithmetic { dst, src, .. }
        | Shift { dst, src, .. }
        | StoreArithmetic { dst, src, .. }
        | LoadArithmetic { dst, src, .. } => {
            let mut uses = Vec::new();
            for val in [dst, src] {
                if is_gp_variable(val) {
                    uses.push(val.clone());
                }
            }
            uses
        }

        StackArg { .. } | Label(_) | Goto(_) | Input => Vec::new(),

        Compare { lhs, rhs, .. } | CJump { lhs, rhs, .. } => {
            let mut uses = Vec::new();
            for val in [lhs, rhs] {
                if is_gp_variable(val) {
                    uses.push(val.clone());
                }
            }
            uses
        }

        Return => {
            let result_and_callee_save = [RAX, R12, R13, R14, R15, RBP, RBX];
            result_and_callee_save
                .into_iter()
                .map(Value::Register)
                .collect()
        }

        Call { callee, args } => {
            let args = *args;
            let mut uses = Vec::new();
            if is_gp_variable(callee) {
                uses.push(callee.clone());
            }
            if args >= 1 {
                uses.push(Value::Register(RDI));
            }
            if args >= 2 {
                uses.push(Value::Register(RSI));
            }
            if args >= 3 {
                uses.push(Value::Register(RDX));
            }
            if args >= 4 {
                uses.push(Value::Register(RCX));
            }
            if args >= 5 {
                uses.push(Value::Register(R8));
            }
            if args >= 6 {
                uses.push(Value::Register(R9));
            }
            uses
        }

        Print => vec![Value::Register(RDI)],

        Allocate => vec![Value::Register(RDI), Value::Register(RSI)],

        TupleError => vec![
            Value::Register(RDI),
            Value::Register(RSI),
            Value::Register(RDX),
        ],

        TensorError(args) => {
            let args = *args;
            let mut uses = Vec::new();
            if args >= 1 {
                uses.push(Value::Register(RDI));
            }
            if args >= 3 {
                uses.extend_from_slice(&[Value::Register(RSI), Value::Register(RDX)]);
            }
            if args == 4 {
                uses.push(Value::Register(RCX));
            }
            uses
        }

        Increment(val) | Decrement(val) => vec![val.clone()],

        LEA { src, offset, .. } => vec![src.clone(), offset.clone()],
    }
}

fn defs(inst: &Instruction) -> Vec<Value> {
    use Instruction::*;
    use Register::*;

    match inst {
        Assign { dst, .. }
        | Load { dst, .. }
        | StackArg { dst, .. }
        | Arithmetic { dst, .. }
        | Shift { dst, .. }
        | LoadArithmetic { dst, .. }
        | Compare { dst, .. }
        | LEA { dst, .. } => vec![dst.clone()],

        Store { .. } | StoreArithmetic { .. } | CJump { .. } | Label(_) | Goto(_) | Return => {
            Vec::new()
        }

        Call { .. } | Print | Input | Allocate | TupleError | TensorError(_) => {
            let caller_save = [R10, R11, R8, R9, RAX, RCX, RDI, RDX, RSI];
            caller_save.into_iter().map(Value::Register).collect()
        }

        Increment(val) | Decrement(val) => vec![val.clone()],
    }
}

fn is_gp_variable(val: &Value) -> bool {
    match val {
        Value::Variable(_) => true,
        Value::Register(reg) if !matches!(reg, Register::RSP) => true,
        _ => false,
    }
}

fn empty_dataflow_set(func: &Function) -> Vec<Vec<HashSet<Value>>> {
    func.basic_blocks
        .iter()
        .map(|block| vec![HashSet::new(); block.instructions.len()])
        .collect()
}
