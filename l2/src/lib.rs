use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

pub trait DisplayResolved {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result;

    fn resolved<'a>(&'a self, interner: &'a Interner<String>) -> DisplayResolvedWrapper<'a, Self>
    where
        Self: Sized,
    {
        DisplayResolvedWrapper {
            inner: self,
            interner,
        }
    }
}

pub struct DisplayResolvedWrapper<'a, T: ?Sized> {
    inner: &'a T,
    interner: &'a Interner<String>,
}

impl<'a, T: DisplayResolved + ?Sized> fmt::Display for DisplayResolvedWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt_with(f, self.interner)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Register {
    RAX,
    RDI,
    RSI,
    RDX,
    R8,
    R9,
    RCX,
    RSP,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    RBP,
    RBX,
}

impl Register {
    pub const GP_REGISTERS: &[Self] = &[
        Register::RDI,
        Register::RSI,
        Register::RDX,
        Register::RCX,
        Register::R8,
        Register::R9,
        Register::RAX,
        Register::R10,
        Register::R11,
        Register::R12,
        Register::R13,
        Register::R14,
        Register::R15,
        Register::RBP,
        Register::RBX,
    ];
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Register::*;

        let reg = match self {
            RAX => "rax",
            RDI => "rdi",
            RSI => "rsi",
            RDX => "rdx",
            R8 => "r8",
            R9 => "r9",
            RCX => "rcx",
            RSP => "rsp",
            R10 => "r10",
            R11 => "r11",
            R12 => "r12",
            R13 => "r13",
            R14 => "r14",
            R15 => "r15",
            RBP => "rbp",
            RBX => "rbx",
        };
        write!(f, "{}", reg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Register(Register),
    Number(i64),
    Label(SymbolId),
    Function(SymbolId),
    Variable(SymbolId),
}

impl Value {
    pub fn is_gp_variable(&self) -> bool {
        match self {
            Value::Variable(_) => true,
            Value::Register(reg) if !matches!(reg, Register::RSP) => true,
            _ => false,
        }
    }
}

impl DisplayResolved for Value {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        match self {
            Self::Register(reg) => write!(f, "{}", reg),
            Self::Number(num) => write!(f, "{}", num),
            Self::Label(id) => write!(f, ":{}", interner.resolve(id.0)),
            Self::Function(id) => write!(f, "@{}", interner.resolve(id.0)),
            Self::Variable(id) => write!(f, "%{}", interner.resolve(id.0)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone)]
pub enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

impl fmt::Display for ArithmeticOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let op = match self {
            Self::PlusEq => "+=",
            Self::MinusEq => "-=",
            Self::MultEq => "*=",
            Self::AndEq => "&=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum ShiftOp {
    LeftShiftEq,
    RightShiftEq,
}

impl fmt::Display for ShiftOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let op = match self {
            Self::LeftShiftEq => "<<=",
            Self::RightShiftEq => ">>=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Less,
    LessEq,
    Equal,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let op = match self {
            Self::Less => "<",
            Self::LessEq => "<=",
            Self::Equal => "=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Assign {
        dst: Value,
        src: Value,
    },
    Load {
        dst: Value,
        src: Value,
        offset: i64,
    },
    Store {
        dst: Value,
        offset: i64,
        src: Value,
    },
    StackArg {
        dst: Value,
        offset: i64,
    },
    Arithmetic {
        dst: Value,
        op: ArithmeticOp,
        src: Value,
    },
    Shift {
        dst: Value,
        op: ShiftOp,
        src: Value,
    },
    StoreArithmetic {
        dst: Value,
        offset: i64,
        op: ArithmeticOp,
        src: Value,
    },
    LoadArithmetic {
        dst: Value,
        op: ArithmeticOp,
        src: Value,
        offset: i64,
    },
    Compare {
        dst: Value,
        lhs: Value,
        op: CompareOp,
        rhs: Value,
    },
    CJump {
        lhs: Value,
        op: CompareOp,
        rhs: Value,
        label: SymbolId,
    },
    Label(SymbolId),
    Goto(SymbolId),
    Return,
    Call {
        callee: Value,
        args: i64,
    },
    Print,
    Input,
    Allocate,
    TupleError,
    TensorError(u8),
    Increment(Value),
    Decrement(Value),
    LEA {
        dst: Value,
        src: Value,
        offset: Value,
        scale: u8,
    },
}

impl Instruction {
    pub fn uses(&self) -> Vec<Value> {
        use Instruction::*;
        use Register::*;

        match self {
            Assign { src, .. } | Load { src, .. } => src
                .is_gp_variable()
                .then_some(vec![src.clone()])
                .unwrap_or_default(),

            Store { dst, src, .. }
            | Arithmetic { dst, src, .. }
            | Shift { dst, src, .. }
            | StoreArithmetic { dst, src, .. }
            | LoadArithmetic { dst, src, .. } => {
                let mut uses = Vec::new();
                for val in [dst, src] {
                    if val.is_gp_variable() {
                        uses.push(val.clone());
                    }
                }
                uses
            }

            StackArg { .. } | Label(_) | Goto(_) | Input => Vec::new(),

            Compare { lhs, rhs, .. } | CJump { lhs, rhs, .. } => {
                let mut uses = Vec::new();
                for val in [lhs, rhs] {
                    if val.is_gp_variable() {
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
                if callee.is_gp_variable() {
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

    pub fn defs(&self) -> Vec<Value> {
        use Instruction::*;
        use Register::*;

        match self {
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

    pub fn replace_value(&mut self, old: &Value, new: &Value) {
        use Instruction::*;

        let replace_helper = |val: &mut Value| {
            if val == old {
                *val = new.clone();
            }
        };

        match self {
            Assign { dst, src }
            | Load { dst, src, .. }
            | Store { dst, src, .. }
            | Arithmetic { dst, src, .. }
            | Shift { dst, src, .. }
            | StoreArithmetic { dst, src, .. }
            | LoadArithmetic { dst, src, .. } => {
                replace_helper(dst);
                replace_helper(src);
            }

            StackArg { dst, .. } => {
                replace_helper(dst);
            }

            Compare { dst, lhs, rhs, .. } => {
                replace_helper(dst);
                replace_helper(lhs);
                replace_helper(rhs);
            }

            CJump { lhs, rhs, .. } => {
                replace_helper(lhs);
                replace_helper(rhs);
            }

            Label(_) | Goto(_) | Return | Print | Input | Allocate | TupleError
            | TensorError(_) => (),

            Call { callee, .. } => {
                replace_helper(callee);
            }

            Increment(val) | Decrement(val) => {
                replace_helper(val);
            }

            LEA {
                dst, src, offset, ..
            } => {
                replace_helper(dst);
                replace_helper(src);
                replace_helper(offset);
            }
        }
    }
}

impl DisplayResolved for Instruction {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        use Instruction::*;

        match self {
            Assign { dst, src } => {
                write!(
                    f,
                    "{} <- {}",
                    dst.resolved(interner),
                    src.resolved(interner)
                )
            }
            Load { dst, src, offset } => {
                write!(
                    f,
                    "{} <- mem {} {}",
                    dst.resolved(interner),
                    src.resolved(interner),
                    offset
                )
            }
            Store { dst, offset, src } => {
                write!(
                    f,
                    "mem {} {} <- {}",
                    dst.resolved(interner),
                    offset,
                    src.resolved(interner)
                )
            }
            StackArg { dst, offset } => {
                write!(f, "{} <- stack-arg {}", dst.resolved(interner), offset)
            }
            Arithmetic { dst, op, src } => write!(
                f,
                "{} {} {}",
                dst.resolved(interner),
                op,
                src.resolved(interner)
            ),
            Shift { dst, op, src } => write!(
                f,
                "{} {} {}",
                dst.resolved(interner),
                op,
                src.resolved(interner)
            ),
            StoreArithmetic {
                dst,
                offset,
                op,
                src,
            } => write!(
                f,
                "mem {} {} {} {}",
                dst.resolved(interner),
                offset,
                op,
                src.resolved(interner)
            ),
            LoadArithmetic {
                dst,
                op,
                src,
                offset,
            } => write!(
                f,
                "{} {} mem {} {}",
                dst.resolved(interner),
                op,
                src.resolved(interner),
                offset
            ),
            Compare { dst, lhs, op, rhs } => {
                write!(
                    f,
                    "{} <- {} {} {}",
                    dst.resolved(interner),
                    lhs.resolved(interner),
                    op,
                    rhs.resolved(interner)
                )
            }
            CJump {
                lhs,
                op,
                rhs,
                label,
            } => write!(
                f,
                "cjump {} {} {} :{}",
                lhs.resolved(interner),
                op,
                rhs.resolved(interner),
                interner.resolve(label.0),
            ),
            Label(label) => write!(f, ":{}", interner.resolve(label.0)),
            Goto(label) => write!(f, "goto :{}", interner.resolve(label.0)),
            Return => write!(f, "return"),
            Call { callee, args } => write!(f, "call {} {}", callee.resolved(interner), args),
            Print => write!(f, "call print 1"),
            Input => write!(f, "call input 0"),
            Allocate => write!(f, "call allocate 2"),
            TupleError => write!(f, "call tuple-error 3"),
            TensorError(args) => write!(f, "call tensor-error {}", args),
            Increment(reg) => write!(f, "{}++", reg.resolved(interner)),
            Decrement(reg) => write!(f, "{}--", reg.resolved(interner)),
            LEA {
                dst,
                src,
                offset,
                scale,
            } => write!(
                f,
                "{} @ {} {} {}",
                dst.resolved(interner),
                src.resolved(interner),
                offset.resolved(interner),
                scale
            ),
        }
    }
}

#[derive(Debug)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
}

impl DisplayResolved for BasicBlock {
    fn fmt_with(&self, f: &mut fmt::Formatter, interner: &Interner<String>) -> fmt::Result {
        for inst in &self.instructions {
            writeln!(f, "\t{}", inst.resolved(interner))?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct BlockId(pub usize);

#[derive(Debug)]
pub struct Function {
    pub name: SymbolId,
    pub args: i64,
    pub locals: i64,
    pub basic_blocks: Vec<BasicBlock>,
    pub interner: Interner<String>,
    pub cfg: ControlFlowGraph,
}

impl Function {
    pub fn build(
        name: SymbolId,
        args: i64,
        instructions: Vec<Instruction>,
        interner: Interner<String>,
    ) -> Self {
        let mut basic_blocks = vec![BasicBlock {
            id: BlockId(0),
            instructions: Vec::new(),
        }];

        for inst in instructions {
            let block = basic_blocks.last_mut().unwrap();
            match inst {
                Instruction::CJump { .. }
                | Instruction::Goto(_)
                | Instruction::Return
                | Instruction::TupleError
                | Instruction::TensorError(_) => {
                    block.instructions.push(inst);
                    basic_blocks.push(BasicBlock {
                        id: BlockId(basic_blocks.len()),
                        instructions: Vec::new(),
                    });
                }

                Instruction::Label(_) => {
                    if block.instructions.is_empty() {
                        block.instructions.push(inst);
                    } else {
                        basic_blocks.push(BasicBlock {
                            id: BlockId(basic_blocks.len()),
                            instructions: vec![inst],
                        });
                    }
                }

                _ => block.instructions.push(inst),
            }
        }

        if basic_blocks
            .last()
            .map_or(false, |block| block.instructions.is_empty())
        {
            basic_blocks.pop();
        }

        let cfg = ControlFlowGraph::build(&basic_blocks);

        Self {
            name,
            args,
            locals: 0,
            basic_blocks,
            interner,
            cfg,
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "(@{}\n\t{}",
            self.interner.resolve(self.name.0),
            self.args
        )?;

        for block in &self.basic_blocks {
            write!(f, "{}", block.resolved(&self.interner))?;
        }

        writeln!(f, ")")
    }
}

#[derive(Debug, Default, Clone)]
pub struct Interner<T> {
    map: HashMap<T, usize>,
    vec: Vec<T>,
}

impl<T: Clone + Eq + Hash> Interner<T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            vec: Vec::new(),
        }
    }

    pub fn intern(&mut self, item: T) -> usize {
        if let Some(&index) = self.map.get(&item) {
            index
        } else {
            let index = self.vec.len();
            self.map.insert(item.clone(), index);
            self.vec.push(item);
            index
        }
    }

    pub fn resolve(&self, index: usize) -> &T {
        &self.vec[index]
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn get(&self, value: &T) -> Option<usize> {
        self.map.get(value).copied()
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub successors: Vec<Vec<BlockId>>,
    pub predecessors: Vec<Vec<BlockId>>,
}

impl ControlFlowGraph {
    pub fn build(basic_blocks: &[BasicBlock]) -> Self {
        let label_to_block: HashMap<SymbolId, BlockId> = basic_blocks
            .iter()
            .filter_map(|block| {
                if let Some(Instruction::Label(id)) = block.instructions.first() {
                    Some((id.clone(), block.id.clone()))
                } else {
                    None
                }
            })
            .collect();

        let num_blocks = basic_blocks.len();
        let mut cfg = Self {
            successors: vec![Vec::new(); num_blocks],
            predecessors: vec![Vec::new(); num_blocks],
        };
        let last_index = num_blocks.saturating_sub(1);

        for block in basic_blocks {
            match block.instructions.last() {
                Some(Instruction::CJump { label, .. }) => {
                    let successor = label_to_block
                        .get(label)
                        .unwrap_or_else(|| panic!("invalid label {:?}", label));
                    cfg.successors[block.id.0].push(successor.clone());
                    cfg.predecessors[successor.0].push(block.id.clone());

                    if block.id.0 < last_index && block.id.0 + 1 != successor.0 {
                        cfg.successors[block.id.0].push(BlockId(block.id.0 + 1));
                        cfg.predecessors[block.id.0 + 1].push(block.id.clone());
                    }
                }

                Some(Instruction::Goto(label)) => {
                    let successor = label_to_block
                        .get(label)
                        .unwrap_or_else(|| panic!("invalid label {:?}", label));
                    cfg.successors[block.id.0].push(successor.clone());
                    cfg.predecessors[successor.0].push(block.id.clone());
                }

                Some(Instruction::Return)
                | Some(Instruction::TupleError)
                | Some(Instruction::TensorError(_)) => (),

                Some(_) => {
                    if block.id.0 < last_index {
                        cfg.successors[block.id.0].push(BlockId(block.id.0 + 1));
                        cfg.predecessors[block.id.0 + 1].push(block.id.clone());
                    }
                }

                None => panic!("empty block {:?}", block.id),
            };
        }

        cfg
    }
}

#[derive(Debug)]
pub struct Program {
    pub entry_point: String,
    pub functions: Vec<Function>,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "(@{}", self.entry_point)?;

        for func in &self.functions {
            writeln!(f, "{}", func)?;
        }

        writeln!(f, ")")
    }
}
