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

impl Register {
    pub fn name(&self) -> &'static str {
        match self {
            Register::RAX => "rax",
            Register::RBX => "rbx",
            Register::RBP => "rbp",
            Register::R10 => "r10",
            Register::R11 => "r11",
            Register::R12 => "r12",
            Register::R13 => "r13",
            Register::R14 => "r14",
            Register::R15 => "r15",
            Register::RDI => "rdi",
            Register::RSI => "rsi",
            Register::RDX => "rdx",
            Register::R8 => "r8",
            Register::R9 => "r9",
            Register::RCX => "rcx",
            Register::RSP => "rsp",
        }
    }

    pub fn name_8(&self) -> &'static str {
        match self {
            Register::RAX => "al",
            Register::RBX => "bl",
            Register::RBP => "bpl",
            Register::R10 => "r10b",
            Register::R11 => "r11b",
            Register::R12 => "r12b",
            Register::R13 => "r13b",
            Register::R14 => "r14b",
            Register::R15 => "r15b",
            Register::RDI => "dil",
            Register::RSI => "sil",
            Register::RDX => "dl",
            Register::R8 => "r8b",
            Register::R9 => "r9b",
            Register::RCX => "cl",
            Register::RSP => panic!("rsp cannot be 8 bit"),
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Register(r) => write!(f, "{}", r),
            Value::Number(n) => write!(f, "{}", n),
            Value::Label(s) => write!(f, ":{}", s),
            Value::Function(s) => write!(f, "@{}", s),
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
        lhs: Register,
        op: ArithmeticOp,
        rhs: Value,
    },
    Shift {
        lhs: Register,
        op: ShiftOp,
        rhs: Value,
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Assign { dst: lhs, src: rhs } => write!(f, "{} <- {}", lhs, rhs),
            Instruction::Load { dst, src, offset } => {
                write!(f, "{} <- mem {} {}", dst, src, offset)
            }
            Instruction::Store { dst, offset, src } => {
                write!(f, "mem {} {} <- {}", dst, offset, src)
            }
            Instruction::Arithmetic { lhs, op, rhs } => write!(f, "{} {} {}", lhs, op, rhs),
            Instruction::Shift { lhs, op, rhs } => write!(f, "{} {} {}", lhs, op, rhs),
            Instruction::StoreArithmetic {
                dst,
                offset,
                op,
                src,
            } => write!(f, "mem {} {} {} {}", dst, offset, op, src),
            Instruction::LoadArithmetic {
                dst,
                op,
                src,
                offset,
            } => write!(f, "{} {} mem {} {}", dst, op, src, offset),
            Instruction::Compare { dst, lhs, op, rhs } => {
                write!(f, "{} <- {} {} {}", dst, lhs, op, rhs)
            }
            Instruction::CJump {
                lhs,
                op,
                rhs,
                label,
            } => write!(f, "cjump {} {} {} :{}", lhs, op, rhs, label),
            Instruction::Label(s) => write!(f, ":{}", s),
            Instruction::Goto(s) => write!(f, "goto :{}", s),
            Instruction::Return => write!(f, "return"),
            Instruction::Call { callee, args } => write!(f, "call {} {}", callee, args),
            Instruction::Print => write!(f, "call print 1"),
            Instruction::Input => write!(f, "call input 0"),
            Instruction::Allocate => write!(f, "call allocate 2"),
            Instruction::TupleError => write!(f, "call tuple-error 3"),
            Instruction::TensorError(n) => write!(f, "call tensor-error {}", n),
            Instruction::Increment(r) => write!(f, "{}++", r),
            Instruction::Decrement(r) => write!(f, "{}--", r),
            Instruction::LEA {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.name)?;
        writeln!(f, "\t{} {}", self.args, self.locals)?;

        for inst in &self.instructions {
            writeln!(f, "\t{}", inst)?;
        }

        write!(f, ")")
    }
}

#[derive(Debug)]
pub struct Program {
    pub entry_point: String,
    pub functions: Vec<Function>,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "(@{}", self.entry_point)?;

        for func in &self.functions {
            writeln!(f, "{}", func)?;
            writeln!(f)?;
        }

        write!(f, ")")
    }
}
