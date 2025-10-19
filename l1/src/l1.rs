#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Value {
    Register(Register),
    Number(i64),
    Label(String),
    FunctionCallee(String),
}

#[derive(Debug, Clone)]
pub enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

#[derive(Debug, Clone)]
pub enum ShiftOp {
    LeftShiftEq,
    RightShiftEq,
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Less,
    LessEq,
    Equal,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Assign {
        lhs: Register,
        rhs: Value,
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

#[derive(Debug)]
pub struct Function {
    name: String,
    args: i64,
    locals: i64,
    instructions: Vec<Instruction>,
}

impl Function {
    pub fn new(name: String, args: i64, locals: i64, instructions: Vec<Instruction>) -> Function {
        Self {
            name,
            args,
            locals,
            instructions,
        }
    }
}

#[derive(Debug)]
pub struct Program {
    entry_point: String,
    functions: Vec<Function>,
}

impl Program {
    pub fn new(entry_point: String, functions: Vec<Function>) -> Program {
        Self {
            entry_point,
            functions,
        }
    }
}
