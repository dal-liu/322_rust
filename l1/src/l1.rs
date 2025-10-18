#[derive(Debug)]
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

#[derive(Debug)]
pub enum Value {
    Reg(Register),
    Imm(i64),
    Label(String),
}

#[derive(Debug)]
pub enum ArithmeticOp {
    PlusEq,
    MinusEq,
    MultEq,
    AndEq,
}

#[derive(Debug)]
pub enum ShiftOp {
    LeftShiftEq,
    RightShiftEq,
}

#[derive(Debug)]
pub enum CompareOp {
    Less,
    LessEq,
    Equal,
}

#[derive(Debug)]
pub enum Instruction {
    AssignInst {
        lhs: Register,
        rhs: Value,
    },
    LoadInst {
        dst: Register,
        src: Register,
        offset: i64,
    },
    StoreInst {
        dst: Register,
        offset: i64,
        src: Register,
    },
    ArithmeticInst {
        lhs: Register,
        op: ArithmeticOp,
        rhs: Value,
    },
    ShiftInst {
        lhs: Register,
        op: ShiftOp,
        rhs: Value,
    },
    StoreArithmeticInst {
        dst: Register,
        offset: i64,
        op: ArithmeticOp,
        src: Value,
    },
    LoadArithmeticInst {
        dst: Register,
        op: ArithmeticOp,
        src: Register,
        offset: i64,
    },
    CompareInst {
        dst: Register,
        lhs: Value,
        op: CompareOp,
        rhs: Value,
    },
    CJumpInst {
        lhs: Value,
        op: CompareOp,
        rhs: Value,
        label: String,
    },
    LabelInst(String),
    GotoInst(String),
    ReturnInst,
    CallInst {
        callee: Value,
        args: i64,
    },
    PrintInst,
    InputInst,
    AllocateInst,
    TupleErrorInst,
    TensorErrorInst(u8),
    IncrementInst(Register),
    DecrementInst(Register),
    LEAInst {
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
