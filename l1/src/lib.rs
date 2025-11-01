use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Register {
    RAX,
    RBX,
    RBP,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    RDI,
    RSI,
    RDX,
    R8,
    R9,
    RCX,
    RSP,
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Register::*;

        let reg = match self {
            RAX => "rax",
            RBX => "rbx",
            RBP => "rbp",
            R10 => "r10",
            R11 => "r11",
            R12 => "r12",
            R13 => "r13",
            R14 => "r14",
            R15 => "r15",
            RDI => "rdi",
            RSI => "rsi",
            RDX => "rdx",
            R8 => "r8",
            R9 => "r9",
            RCX => "rcx",
            RSP => "rsp",
        };
        write!(f, "{}", reg)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Register(Register),
    Number(i64),
    Label(String),
    Function(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Register(reg) => write!(f, "{}", reg),
            Self::Number(num) => write!(f, "{}", num),
            Self::Label(label) => write!(f, ":{}", label),
            Self::Function(callee) => write!(f, "@{}", callee),
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
        dst: Register,
        src: Value,
    },
    Load {
        dst: Register,
        src: Register,
        offset: i64,
    },
    Store {
        dst: Register,
        offset: i64,
        src: Value,
    },
    Arithmetic {
        dst: Register,
        op: ArithmeticOp,
        src: Value,
    },
    Shift {
        dst: Register,
        op: ShiftOp,
        src: Value,
    },
    StoreArithmetic {
        dst: Register,
        offset: i64,
        op: ArithmeticOp,
        src: Value,
    },
    LoadArithmetic {
        dst: Register,
        op: ArithmeticOp,
        src: Register,
        offset: i64,
    },
    Compare {
        dst: Register,
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
    Increment(Register),
    Decrement(Register),
    LEA {
        dst: Register,
        src: Register,
        offset: Register,
        scale: u8,
    },
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Instruction::*;

        match self {
            Assign { dst, src } => write!(f, "{} <- {}", dst, src),
            Load { dst, src, offset } => {
                write!(f, "{} <- mem {} {}", dst, src, offset)
            }
            Store { dst, offset, src } => {
                write!(f, "mem {} {} <- {}", dst, offset, src)
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
pub struct Function {
    pub name: String,
    pub args: i64,
    pub locals: i64,
    pub instructions: Vec<Instruction>,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "(@{}\n\t{} {}", self.name, self.args, self.locals)?;

        for inst in &self.instructions {
            writeln!(f, "\t{}", inst)?;
        }

        writeln!(f, ")")
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
