use std::fmt;

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
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reg = match self {
            Register::RAX => "rax",
            Register::RDI => "rdi",
            Register::RSI => "rsi",
            Register::RDX => "rdx",
            Register::R8 => "r8",
            Register::R9 => "r9",
            Register::RCX => "rcx",
            Register::RSP => "rsp",
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
    Variable(String),
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
        lhs: Value,
        op: ArithmeticOp,
        rhs: Value,
    },
    Shift {
        lhs: Value,
        op: ShiftOp,
        rhs: Value,
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

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Assign { dst, src } => write!(f, "{} <- {}", dst, src),
            Instruction::Load { dst, src, offset } => {
                write!(f, "{} <- mem {} {}", dst, src, offset)
            }
            Instruction::Store { dst, offset, src } => {
                write!(f, "mem {} {} <- {}", dst, offset, src)
            }
            Instruction::StackArg { dst, offset } => {
                write!(f, "{} <- stack-arg {}", dst, offset)
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
            Instruction::Label(label) => write!(f, ":{}", label),
            Instruction::Goto(label) => write!(f, "goto :{}", label),
            Instruction::Return => write!(f, "return"),
            Instruction::Call { callee, args } => write!(f, "call {} {}", callee, args),
            Instruction::Print => write!(f, "call print 1"),
            Instruction::Input => write!(f, "call input 0"),
            Instruction::Allocate => write!(f, "call allocate 2"),
            Instruction::TupleError => write!(f, "call tuple-error 3"),
            Instruction::TensorError(args) => write!(f, "call tensor-error {}", args),
            Instruction::Increment(reg) => write!(f, "{}++", reg),
            Instruction::Decrement(reg) => write!(f, "{}--", reg),
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
pub enum Next {
    Label(Option<String>),
    Index(Vec<usize>),
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub next: Next,
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
