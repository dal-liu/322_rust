use std::collections::HashMap;
use std::fmt;

const MAX_SUCCESSORS: usize = 2;

pub type SymbolId = usize;
pub type BlockId = usize;

#[derive(Debug, Clone)]
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

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Debug, Default)]
pub struct Interner {
    map: HashMap<String, SymbolId>,
    vec: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.map.get(name) {
            id
        } else {
            let id = self.vec.len();
            self.map.insert(name.to_string(), id);
            self.vec.push(name.to_string());
            id
        }
    }

    pub fn resolve(&self, id: SymbolId) -> &str {
        &self.vec[id]
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Register(Register),
    Number(i64),
    Label(SymbolId),
    Function(SymbolId),
    Variable(SymbolId),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Register(r) => write!(f, "{}", r),
            Value::Number(n) => write!(f, "{}", n),
            Value::Label(s) => write!(f, ":{}", s),
            Value::Function(s) => write!(f, "@{}", s),
            Value::Variable(s) => write!(f, "%{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

impl fmt::Display for ArithmeticOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            ArithmeticOp::PlusEq => "+=",
            ArithmeticOp::MinusEq => "-=",
            ArithmeticOp::MultEq => "*=",
            ArithmeticOp::AndEq => "&=",
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            ShiftOp::LeftShiftEq => "<<=",
            ShiftOp::RightShiftEq => ">>=",
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            CompareOp::Less => "<",
            CompareOp::LessEq => "<=",
            CompareOp::Equal => "=",
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
        label: String,
    },
    Label(String),
    Goto(String),
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
    // pub fn uses(&self) -> Vec<Value> {
    //     use Instruction::*;
    //     match self {
    //         Assign { src, .. } | Load { src, .. } => {
    //             if matches!(src, Value::Register(_) | Value::Variable(_)) {
    //                 vec![src.clone()]
    //             } else {
    //                 vec![]
    //             }
    //         }
    //         Store { dst, src, .. }
    //         | Arithmetic { dst, src, .. }
    //         | Shift { dst, src, .. }
    //         | StoreArithmetic { dst, src, .. }
    //         | LoadArithmetic { dst, src, .. } => {
    //             let mut uses = vec![];
    //             if let Value::Variable(_) = dst {
    //                 uses.push(dst.clone());
    //             }
    //             vec![dst.clone(), src.clone()]
    //         }
    //         Compare { lhs, rhs, .. } | CJump { lhs, rhs, .. } => {
    //             vec![lhs.clone(), rhs.clone()]
    //         }
    //         Increment(val) | Decrement(val) => vec![val.clone()],
    //     }
    // }

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

            Increment(val) | Decrement(val) => vec![val.clone()],

            Call { .. } | Print | Input | Allocate | TupleError | TensorError(_) => {
                let callee_save = [R10, R11, R8, R9, RAX, RCX, RDI, RDX, RSI];
                callee_save.into_iter().map(Value::Register).collect()
            }

            Store { .. } | StoreArithmetic { .. } | CJump { .. } | Label(_) | Goto(_) | Return => {
                vec![]
            }
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Instruction::*;
        match self {
            Assign { dst, src } => write!(f, "{} <- {}", dst, src),
            Load { dst, src, offset } => {
                write!(f, "{} <- mem {} {}", dst, src, offset)
            }
            Store { dst, offset, src } => {
                write!(f, "mem {} {} <- {}", dst, offset, src)
            }
            StackArg { dst, offset } => {
                write!(f, "{} <- stack-arg {}", dst, offset)
            }
            Arithmetic { dst, op, src } => write!(f, "{} {} {}", dst, op, src),
            Shift { dst, op, src } => write!(f, "{} {} {}", dst, op, src),
            StoreArithmetic {
                dst,
                offset,
                op,
                src,
            } => write!(f, "mem {} {} {} {}", dst, offset, op, src),
            LoadArithmetic {
                dst,
                op,
                src,
                offset,
            } => write!(f, "{} {} mem {} {}", dst, op, src, offset),
            Compare { dst, lhs, op, rhs } => {
                write!(f, "{} <- {} {} {}", dst, lhs, op, rhs)
            }
            CJump {
                lhs,
                op,
                rhs,
                label,
            } => write!(f, "cjump {} {} {} :{}", lhs, op, rhs, label),
            Label(label) => write!(f, ":{}", label),
            Goto(label) => write!(f, "goto :{}", label),
            Return => write!(f, "return"),
            Call { callee, args } => write!(f, "call {} {}", callee, args),
            Print => write!(f, "call print 1"),
            Input => write!(f, "call input 0"),
            Allocate => write!(f, "call allocate 2"),
            TupleError => write!(f, "call tuple-error 3"),
            TensorError(args) => write!(f, "call tensor-error {}", args),
            Increment(reg) => write!(f, "{}++", reg),
            Decrement(reg) => write!(f, "{}--", reg),
            LEA {
                dst,
                src,
                offset,
                scale,
            } => write!(f, "{} @ {} {} {}", dst, src, offset, scale),
        }
    }
}

#[derive(Debug)]
pub enum Target {
    Label(Option<String>),
    Indexes([Option<usize>; MAX_SUCCESSORS]),
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub target: Target,
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for inst in &self.instructions {
            writeln!(f, "\t{}", inst)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub args: i64,
    pub basic_blocks: Vec<BasicBlock>,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.name)?;
        writeln!(f, "\t{}", self.args)?;

        for block in &self.basic_blocks {
            write!(f, "{}", block)?;
        }

        writeln!(f, ")")
    }
}

#[derive(Debug)]
pub struct Program {
    pub entry_point: String,
    pub functions: Vec<Function>,
    pub interner: Interner,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.entry_point)?;

        for func in &self.functions {
            writeln!(f, "{}", func)?;
        }

        writeln!(f, ")")
    }
}
